//! Test tooling: golden files, a process runner and a framed RAP client.
//!
//! Never shipped. Owning unit: U1 walking-skeleton (extended by U2
//! acceptance-harness).

#![forbid(unsafe_code)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::fmt;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Deadline for any process a test starts (performance design).
pub const PROCESS_DEADLINE: Duration = Duration::from_secs(60);

/// The repository root (two levels above this crate).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

/// A directory under the system temporary directory, removed on drop.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates a new, empty, uniquely named directory.
    pub fn new(label: &str) -> TempDir {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!("rail-{label}-{}-{n}-{nanos}", std::process::id());
        let path = std::env::temp_dir().join(name);
        fs::create_dir_all(&path).expect("create temporary directory");
        TempDir { path }
    }

    /// The directory's path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Recursively copies the contents of `src` into `dst`.
pub fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    let mut entries = fs::read_dir(src)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Compares `actual` with the committed golden file at `path`. With
/// `RAIL_BLESS=1` the file is rewritten instead (NFR9.4).
pub fn assert_golden(path: &Path, actual: &str) {
    let bless = std::env::var("RAIL_BLESS").is_ok_and(|v| v == "1");
    if let Err(message) = check_golden(path, actual, bless) {
        panic!("{message}");
    }
}

/// The logic behind [`assert_golden`], with the bless switch passed in.
pub fn check_golden(path: &Path, actual: &str, bless: bool) -> Result<(), String> {
    if bless {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        return fs::write(path, actual).map_err(|e| format!("{}: {e}", path.display()));
    }
    let expected = fs::read_to_string(path).map_err(|e| {
        format!(
            "golden file {} is missing or unreadable ({e}); run with RAIL_BLESS=1 to create it",
            path.display()
        )
    })?;
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "output differs from golden file {}\n--- expected\n{expected}\n--- actual\n{actual}\n(run with RAIL_BLESS=1 to accept the new output)",
            path.display()
        ))
    }
}

/// What a finished child process produced.
#[derive(Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    /// The exit code, when the process exited normally.
    pub code: Option<i32>,
    /// The signal number, when the process was ended by a signal.
    pub signal: Option<i32>,
    /// Captured standard output.
    pub stdout: Vec<u8>,
    /// Captured standard error.
    pub stderr: Vec<u8>,
}

impl ProcessOutput {
    /// Standard output as text (lossy).
    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    /// Standard error as text (lossy).
    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    fn from_status(status: ExitStatus, stdout: Vec<u8>, stderr: Vec<u8>) -> ProcessOutput {
        use std::os::unix::process::ExitStatusExt;
        ProcessOutput {
            code: status.code(),
            signal: status.signal(),
            stdout,
            stderr,
        }
    }
}

impl fmt::Debug for ProcessOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessOutput")
            .field("code", &self.code)
            .field("signal", &self.signal)
            .field("stdout", &self.stdout_str())
            .field("stderr", &self.stderr_str())
            .finish()
    }
}

/// The process ran past its deadline and was killed.
#[derive(Debug)]
pub struct DeadlineExceeded {
    /// The command line that was running.
    pub command: String,
    /// What the process wrote before it was killed.
    pub partial: ProcessOutput,
}

fn describe(cmd: &Command) -> String {
    let mut text = cmd.get_program().to_string_lossy().into_owned();
    for arg in cmd.get_args() {
        text.push(' ');
        text.push_str(&arg.to_string_lossy());
    }
    text
}

fn spawn_reader<R: Read + Send + 'static>(mut source: R) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = source.read_to_end(&mut buffer);
        buffer
    })
}

/// Runs `cmd` with `stdin` as its input and a [`PROCESS_DEADLINE`]; panics
/// with the command line and captured output if the deadline passes.
pub fn run_process(cmd: &mut Command, stdin: &[u8]) -> ProcessOutput {
    match run_process_with_deadline(cmd, stdin, PROCESS_DEADLINE) {
        Ok(output) => output,
        Err(e) => panic!(
            "process exceeded its {}s deadline: {}\n{:?}",
            PROCESS_DEADLINE.as_secs(),
            e.command,
            e.partial
        ),
    }
}

/// Runs `cmd` with `stdin` as its input, killing it after `deadline`.
pub fn run_process_with_deadline(
    cmd: &mut Command,
    stdin: &[u8],
    deadline: Duration,
) -> Result<ProcessOutput, DeadlineExceeded> {
    let command = describe(cmd);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot start {command}: {e}"));
    let input = stdin.to_vec();
    let mut child_stdin = child.stdin.take().expect("stdin pipe");
    let writer = thread::spawn(move || {
        let _ = child_stdin.write_all(&input);
    });
    let out = spawn_reader(child.stdout.take().expect("stdout pipe"));
    let err = spawn_reader(child.stderr.take().expect("stderr pipe"));
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for child") {
            break Some(status);
        }
        if start.elapsed() >= deadline {
            let _ = child.kill();
            break None;
        }
        thread::sleep(Duration::from_millis(5));
    };
    let status = status.unwrap_or_else(|| child.wait().expect("wait for killed child"));
    let _ = writer.join();
    let output = ProcessOutput::from_status(
        status,
        out.join().unwrap_or_default(),
        err.join().unwrap_or_default(),
    );
    if start.elapsed() >= deadline && output.signal.is_some() {
        return Err(DeadlineExceeded {
            command,
            partial: output,
        });
    }
    Ok(output)
}

/// Frames a message body with its `Content-Length` header (contract E1).
pub fn frame(body: &[u8]) -> Vec<u8> {
    let mut framed = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    framed.extend_from_slice(body);
    framed
}

/// Reads one strictly framed message: exactly one `Content-Length` header.
/// Returns `Ok(None)` at a clean end of input.
fn read_frame<R: BufRead>(reader: &mut R) -> Result<Option<Vec<u8>>, String> {
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) if header.is_empty() => return Ok(None),
            Ok(0) => return Err(format!("end of output inside a header: {header:?}")),
            Ok(_) => header.push(byte[0]),
            Err(e) => return Err(e.to_string()),
        }
        if header.len() > 1024 {
            return Err(format!("non-frame bytes on stdout: {header:?}"));
        }
    }
    let text = String::from_utf8(header).map_err(|e| e.to_string())?;
    let length = text
        .strip_prefix("Content-Length: ")
        .and_then(|rest| rest.strip_suffix("\r\n\r\n"))
        .and_then(|digits| digits.parse::<usize>().ok())
        .ok_or_else(|| format!("not a frame header: {text:?}"))?;
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    Ok(Some(body))
}

/// A client for a `rail rap` child process.
pub struct RapClient {
    child: Child,
    stdin: Option<ChildStdin>,
    frames: Receiver<Result<Vec<u8>, String>>,
    stderr: Option<thread::JoinHandle<Vec<u8>>>,
}

impl RapClient {
    /// Starts `<program> rap` in `cwd`, with logging off.
    pub fn spawn(program: &Path, cwd: &Path) -> RapClient {
        let mut cmd = Command::new(program);
        cmd.arg("rap").current_dir(cwd).env_remove("RAIL_LOG");
        RapClient::spawn_command(cmd)
    }

    /// Starts an already configured server command.
    pub fn spawn_command(mut cmd: Command) -> RapClient {
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot start {}: {e}", describe(&cmd)));
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout pipe");
        let stderr = spawn_reader(child.stderr.take().expect("stderr pipe"));
        let (tx, frames) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_frame(&mut reader) {
                    Ok(Some(body)) => {
                        if tx.send(Ok(body)).is_err() {
                            return;
                        }
                    }
                    Ok(None) => return,
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                }
            }
        });
        RapClient {
            child,
            stdin,
            frames,
            stderr: Some(stderr),
        }
    }

    /// The server's process ID.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Sends one framed message.
    pub fn send(&mut self, body: &str) {
        self.send_raw(&frame(body.as_bytes()));
    }

    /// Sends raw bytes, framed or not.
    pub fn send_raw(&mut self, bytes: &[u8]) {
        let stdin = self.stdin.as_mut().expect("server stdin is open");
        stdin.write_all(bytes).expect("write to server");
        stdin.flush().expect("flush to server");
    }

    /// Receives the next framed response body, waiting at most
    /// [`PROCESS_DEADLINE`].
    pub fn recv(&mut self) -> String {
        match self.try_recv(PROCESS_DEADLINE) {
            Some(body) => body,
            None => panic!("no response from the server"),
        }
    }

    /// Receives the next response within `wait`, or `None` if none arrives.
    /// Panics if the server writes anything that is not a frame.
    pub fn try_recv(&mut self, wait: Duration) -> Option<String> {
        match self.frames.recv_timeout(wait) {
            Ok(Ok(body)) => Some(String::from_utf8(body).expect("UTF-8 response")),
            Ok(Err(e)) => panic!("protocol stream corrupted: {e}"),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => None,
        }
    }

    /// Closes the server's input and waits for it to exit.
    pub fn close(mut self) -> ProcessOutput {
        drop(self.stdin.take());
        let start = Instant::now();
        let status = loop {
            if let Some(status) = self.child.try_wait().expect("wait for server") {
                break status;
            }
            if start.elapsed() >= PROCESS_DEADLINE {
                let _ = self.child.kill();
                panic!("server did not exit after end of input");
            }
            thread::sleep(Duration::from_millis(5));
        };
        let stderr = self
            .stderr
            .take()
            .map(|h| h.join().unwrap_or_default())
            .unwrap_or_default();
        let mut leftover = Vec::new();
        while let Ok(Ok(body)) = self.frames.try_recv() {
            leftover.extend_from_slice(&body);
        }
        ProcessOutput::from_status(status, leftover, stderr)
    }
}

impl Drop for RapClient {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
