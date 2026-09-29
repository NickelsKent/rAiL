//! Running a built program as a separate, bounded child process (NFR5.1,
//! NFR5.5, NFR5.6, NFR9.9, BR6.5, BR6.6).

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rail_json::Value;

use crate::{ErrorCode, ToolError};

/// Limits for one program run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunLimits {
    /// Wall-clock limit.
    pub time: Duration,
    /// Cap on standard output and standard error together.
    pub output_bytes: usize,
}

impl Default for RunLimits {
    fn default() -> RunLimits {
        RunLimits {
            time: Duration::from_secs(10),
            output_bytes: 16 * 1024 * 1024,
        }
    }
}

/// What one run of a program produced (returned at the end, BR6.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunResult {
    /// The program's exit code.
    pub exit_code: i32,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

impl RunResult {
    /// `{exit_code, stdout, stderr}`.
    pub fn to_json(&self) -> Value {
        Value::object([
            ("exit_code", Value::int(i64::from(self.exit_code))),
            ("stdout", Value::str(&self.stdout)),
            ("stderr", Value::str(&self.stderr)),
        ])
    }
}

fn run_failed(message: impl Into<String>) -> ToolError {
    ToolError::new(ErrorCode::RunFailed, message)
}

/// Shared capture budget for both pipes.
struct Budget {
    used: AtomicUsize,
    exceeded: AtomicBool,
    cap: usize,
}

fn capture<R: Read + Send + 'static>(mut pipe: R, budget: Arc<Budget>) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut kept = Vec::new();
        let mut chunk = vec![0u8; 64 * 1024];
        loop {
            let n = match pipe.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let before = budget.used.fetch_add(n, Ordering::SeqCst);
            if before + n > budget.cap {
                budget.exceeded.store(true, Ordering::SeqCst);
                break;
            }
            kept.extend_from_slice(chunk.get(..n).unwrap_or(&[]));
        }
        kept
    })
}

/// Runs the program at `path` with no arguments, empty standard input, a
/// cleared environment and captured output, within `limits`.
pub fn run_executable(path: &Path, limits: &RunLimits) -> Result<RunResult, ToolError> {
    let mut child = Command::new(path)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| run_failed(format!("could not start: {}", e.kind())))?;
    let budget = Arc::new(Budget {
        used: AtomicUsize::new(0),
        exceeded: AtomicBool::new(false),
        cap: limits.output_bytes,
    });
    let (Some(out_pipe), Some(err_pipe)) = (child.stdout.take(), child.stderr.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(run_failed("could not start: no output pipes"));
    };
    let out = capture(out_pipe, Arc::clone(&budget));
    let err = capture(err_pipe, Arc::clone(&budget));
    let outcome = wait(&mut child, limits.time, &budget);
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    let status = outcome?;
    if budget.exceeded.load(Ordering::SeqCst) {
        return Err(run_failed("output limit exceeded"));
    }
    exit_code(status).map(|exit_code| RunResult {
        exit_code,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

/// Waits for the child, killing it at the time or output limit.
fn wait(child: &mut Child, time: Duration, budget: &Budget) -> Result<ExitStatus, ToolError> {
    let start = Instant::now();
    loop {
        if budget.exceeded.load(Ordering::SeqCst) {
            kill(child);
            return Err(run_failed("output limit exceeded"));
        }
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(e) => {
                kill(child);
                return Err(run_failed(format!(
                    "could not wait for the program: {}",
                    e.kind()
                )));
            }
        }
        if start.elapsed() >= time {
            kill(child);
            return Err(run_failed("time limit exceeded"));
        }
        thread::sleep(Duration::from_millis(2));
    }
}

fn kill(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn exit_code(status: ExitStatus) -> Result<i32, ToolError> {
    use std::os::unix::process::ExitStatusExt;
    if let Some(code) = status.code() {
        return Ok(code);
    }
    let name = status
        .signal()
        .map_or_else(|| "unknown".to_string(), signal_name);
    Err(run_failed(format!("ended by signal {name}")))
}

fn signal_name(signal: i32) -> String {
    let bus = if cfg!(target_os = "macos") { 10 } else { 7 };
    let name = match signal {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL",
        5 => "SIGTRAP",
        6 => "SIGABRT",
        8 => "SIGFPE",
        9 => "SIGKILL",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        s if s == bus => "SIGBUS",
        s => return format!("{s}"),
    };
    name.to_string()
}
