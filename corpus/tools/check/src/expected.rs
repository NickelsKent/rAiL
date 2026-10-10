//! Expected outputs, bless mode and comparison (BR2.2, BR6.2, BR7.2).

use crate::model::Lang;
use crate::proc::ProcOutcome;
use std::fs;
use std::io;
use std::path::Path;

/// The known-good result for one fixed input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expected {
    pub stdout: Vec<u8>,
    /// `Some` exactly when the exit code is 1: the one typed-error line.
    pub stderr: Option<Vec<u8>>,
    /// The run directory's final tree (io-services only).
    pub tree: Option<String>,
}

/// Whether the check compares or the Rust version writes `expected/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Check,
    Bless,
}

/// The result of judging one run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Blessed,
    Fail(String),
}

/// Turns a finished run into an `Expected`, or says why it cannot be one:
/// exit 0 with empty stderr, or exit 1 with exactly one line
/// `error: <kind>: <detail>` (BR6.2).
pub fn expected_from_outcome(
    outcome: &ProcOutcome,
    tree: Option<String>,
) -> Result<Expected, String> {
    if outcome.timed_out {
        return Err("the run timed out".to_string());
    }
    if outcome.output_truncated {
        return Err("the run wrote too much output".to_string());
    }
    if let Some(signal) = outcome.signal {
        return Err(format!("the run was killed by signal {signal}"));
    }
    match outcome.code {
        Some(0) if outcome.stderr.is_empty() => Ok(Expected {
            stdout: outcome.stdout.clone(),
            stderr: None,
            tree,
        }),
        Some(0) => Err("the run exited 0 but wrote to stderr".to_string()),
        Some(1) => {
            typed_error_line(&outcome.stderr)?;
            Ok(Expected {
                stdout: outcome.stdout.clone(),
                stderr: Some(outcome.stderr.clone()),
                tree,
            })
        }
        Some(other) => Err(format!(
            "the run exited with code {other}; only 0 and 1 are allowed"
        )),
        None => Err("the run has no exit code".to_string()),
    }
}

/// BR6.2: exactly one line `error: <kind>: <detail>`, no absolute paths.
fn typed_error_line(stderr: &[u8]) -> Result<(), String> {
    let text = std::str::from_utf8(stderr).map_err(|_| "stderr is not UTF-8".to_string())?;
    let Some(line) = text.strip_suffix('\n') else {
        return Err("stderr must be one line ending in a newline".to_string());
    };
    if line.contains('\n') {
        return Err("stderr must be exactly one line".to_string());
    }
    let shape = line
        .strip_prefix("error: ")
        .and_then(|rest| rest.split_once(": "));
    let Some((kind, detail)) = shape else {
        return Err("stderr must be `error: <kind>: <detail>`".to_string());
    };
    let kind_ok = !kind.is_empty()
        && kind
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !kind_ok || detail.is_empty() {
        return Err("stderr must be `error: <kind>: <detail>` with a lowercase kind".to_string());
    }
    let absolute = detail
        .split_whitespace()
        .any(|t| t.trim_start_matches(['\'', '"', '`', '(']).starts_with('/'));
    if absolute {
        return Err("the error detail contains an absolute path".to_string());
    }
    Ok(())
}

/// Reads `expected/<input>.out` / `.err` / `.tree`.
pub fn load_expected(program_dir: &Path, input: &str, want_tree: bool) -> Result<Expected, String> {
    let dir = program_dir.join("expected");
    let read = |ext: &str| fs::read(dir.join(format!("{input}.{ext}")));
    let stdout = read("out").map_err(|e| format!("expected/{input}.out: {e}"))?;
    let stderr = match read("err") {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(format!("expected/{input}.err: {e}")),
    };
    let tree = if want_tree {
        let bytes = read("tree").map_err(|e| format!("expected/{input}.tree: {e}"))?;
        Some(String::from_utf8(bytes).map_err(|_| format!("expected/{input}.tree is not UTF-8"))?)
    } else {
        None
    };
    Ok(Expected {
        stdout,
        stderr,
        tree,
    })
}

/// Writes `expected/<input>.out`, `.err` (only for exit 1) and `.tree`.
pub fn write_expected(program_dir: &Path, input: &str, expected: &Expected) -> io::Result<()> {
    let dir = program_dir.join("expected");
    fs::create_dir_all(&dir)?;
    let path = |ext: &str| dir.join(format!("{input}.{ext}"));
    fs::write(path("out"), &expected.stdout)?;
    for (ext, content) in [
        ("err", expected.stderr.as_deref()),
        ("tree", expected.tree.as_deref().map(str::as_bytes)),
    ] {
        match content {
            Some(bytes) => fs::write(path(ext), bytes)?,
            None => match fs::remove_file(path(ext)) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            },
        }
    }
    Ok(())
}

/// Judges one run: in bless mode the Rust version writes `expected/` and the
/// other languages are compared with it; in check mode nothing is written.
pub fn judge(
    mode: Mode,
    lang: Lang,
    program_dir: &Path,
    input: &str,
    actual: &Expected,
) -> Verdict {
    if mode == Mode::Bless && lang == Lang::Rust {
        return match write_expected(program_dir, input, actual) {
            Ok(()) => Verdict::Blessed,
            Err(e) => Verdict::Fail(format!("cannot write expected/{input}: {e}")),
        };
    }
    match load_expected(program_dir, input, actual.tree.is_some()) {
        Err(message) => Verdict::Fail(format!("no expected output: {message}")),
        Ok(expected) => match compare(&expected, actual) {
            None => Verdict::Pass,
            Some(message) => Verdict::Fail(message),
        },
    }
}

/// Why `actual` is not `expected`, naming the first difference.
fn compare(expected: &Expected, actual: &Expected) -> Option<String> {
    if let Some(m) = compare_bytes("stdout", &expected.stdout, &actual.stdout) {
        return Some(m);
    }
    match (&expected.stderr, &actual.stderr) {
        (None, None) => {}
        (Some(want), Some(got)) => {
            if let Some(m) = compare_bytes("stderr", want, got) {
                return Some(m);
            }
        }
        (None, Some(got)) => {
            return Some(format!(
                "the run failed with `{}` but exit code 0 was expected",
                String::from_utf8_lossy(got).trim_end()
            ));
        }
        (Some(want), None) => {
            return Some(format!(
                "the run succeeded but `{}` (exit code 1) was expected",
                String::from_utf8_lossy(want).trim_end()
            ));
        }
    }
    if expected.tree != actual.tree {
        let want = expected.tree.as_deref().unwrap_or("");
        let got = actual.tree.as_deref().unwrap_or("");
        let (mut w, mut g) = (want.lines(), got.lines());
        loop {
            match (w.next(), g.next()) {
                (Some(a), Some(b)) if a == b => {}
                (Some(a), Some(b)) => {
                    return Some(format!(
                        "the run directory tree differs: expected `{a}` but found `{b}`"
                    ));
                }
                (Some(a), None) => {
                    return Some(format!("the run directory tree lacks `{a}`"));
                }
                (None, Some(b)) => {
                    return Some(format!("the run directory tree has an extra entry `{b}`"));
                }
                (None, None) => break,
            }
        }
    }
    None
}

fn compare_bytes(stream: &str, want: &[u8], got: &[u8]) -> Option<String> {
    if want == got {
        return None;
    }
    let common = want.len().min(got.len());
    match (0..common).find(|&i| want[i] != got[i]) {
        Some(i) => Some(format!(
            "{stream} differs at byte {i} (expected 0x{:02x}, got 0x{:02x})",
            want[i], got[i]
        )),
        None if got.len() < want.len() => Some(format!(
            "{stream}: output ends at byte {} but the expected output continues",
            got.len()
        )),
        None => Some(format!(
            "{stream} has extra output from byte {} (the expected output ends there)",
            want.len()
        )),
    }
}
