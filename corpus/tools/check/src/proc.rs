//! Starting a process in its own process group, with a deadline and an
//! output limit.

use std::io::{self, Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// The most output kept from one stream; a program that writes more is
/// stopped (a runaway program must not fill the machine's memory).
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

/// What a finished process left behind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProcOutcome {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
    pub output_truncated: bool,
}

/// Kills every process in group `pgid` with `/bin/kill -KILL -- -<pgid>`,
/// started as an argument list (no shell, no `unsafe`).
fn kill_process_group(pgid: u32) -> io::Result<()> {
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

/// Reads `source` until it closes, keeping at most `MAX_OUTPUT_BYTES`; sets
/// `over` and stops reading when the limit is passed.
fn reader<R: Read + Send + 'static>(
    mut source: R,
    over: Arc<AtomicBool>,
) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 64 * 1024];
        loop {
            match source.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buffer.extend_from_slice(&chunk[..n]);
                    if buffer.len() > MAX_OUTPUT_BYTES {
                        buffer.truncate(MAX_OUTPUT_BYTES);
                        over.store(true, Ordering::SeqCst);
                        break;
                    }
                }
            }
        }
        buffer
    })
}

/// Runs `cmd` (stdin from `stdin`, output captured) until it exits or
/// `deadline` passes; the whole process group is killed at the deadline, and
/// also when a stream passes the output limit.
pub fn run_command(mut cmd: Command, stdin: &[u8], deadline: Duration) -> io::Result<ProcOutcome> {
    cmd.process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let pgid = child.id();
    let over = Arc::new(AtomicBool::new(false));
    let input = stdin.to_vec();
    let writer = child.stdin.take().map(|mut pipe| {
        thread::spawn(move || {
            let _ = pipe.write_all(&input);
        })
    });
    let out = child.stdout.take().map(|s| reader(s, over.clone()));
    let err = child.stderr.take().map(|s| reader(s, over.clone()));
    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = kill_process_group(pgid);
                return Err(e);
            }
        }
        let past_deadline = start.elapsed() >= deadline;
        if past_deadline || over.load(Ordering::SeqCst) {
            timed_out = past_deadline && !over.load(Ordering::SeqCst);
            if kill_process_group(pgid).is_err() {
                let _ = child.kill();
            }
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(5));
    };
    // Descendants left behind by a process that exited would hold its
    // output pipes open; the group ends with it.
    let _ = kill_process_group(pgid);
    if let Some(w) = writer {
        let _ = w.join();
    }
    let join =
        |h: Option<thread::JoinHandle<Vec<u8>>>| h.and_then(|h| h.join().ok()).unwrap_or_default();
    Ok(ProcOutcome {
        code: status.code(),
        signal: status.signal(),
        stdout: join(out),
        stderr: join(err),
        timed_out,
        output_truncated: over.load(Ordering::SeqCst),
    })
}
