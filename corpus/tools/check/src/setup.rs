//! Error cases that depend on the machine are created at run time (BR6.5).

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path};
use std::process::Command;

/// One run-time setup step, from `inputs/fixed/<input>.setup`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetupOp {
    /// A file with no permissions (created if the input does not hold it).
    Unreadable(String),
    /// A directory that cannot be written to.
    ReadOnlyDir(String),
}

/// Parses `unreadable <path>` and `unwritable-dir <path>` lines (`#` starts a
/// comment). Paths are relative, without `..`.
pub fn parse_setup(text: &str) -> Result<Vec<SetupOp>, String> {
    let mut ops = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let line_no = i + 1;
        let (verb, path) = line
            .split_once(' ')
            .ok_or_else(|| format!("line {line_no}: expected `<verb> <path>`"))?;
        let path = path.trim();
        let relative = !path.is_empty()
            && Path::new(path)
                .components()
                .all(|c| matches!(c, Component::Normal(_)));
        if !relative {
            return Err(format!(
                "line {line_no}: the path must be relative, without `..`"
            ));
        }
        ops.push(match verb {
            "unreadable" => SetupOp::Unreadable(path.to_string()),
            "unwritable-dir" => SetupOp::ReadOnlyDir(path.to_string()),
            _ => return Err(format!("line {line_no}: unknown setup verb `{verb}`")),
        });
    }
    Ok(ops)
}

/// Creates the cases in `run_dir`.
pub fn apply_setup(run_dir: &Path, ops: &[SetupOp]) -> io::Result<()> {
    for op in ops {
        match op {
            SetupOp::Unreadable(rel) => {
                let path = run_dir.join(rel);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                if !path.exists() {
                    fs::write(&path, b"locked\n")?;
                }
                fs::set_permissions(&path, fs::Permissions::from_mode(0o000))?;
            }
            SetupOp::ReadOnlyDir(rel) => {
                let path = run_dir.join(rel);
                fs::create_dir_all(&path)?;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o555))?;
            }
        }
    }
    Ok(())
}

/// Gives every file and directory under `dir` ordinary permissions again so
/// it can be listed and removed.
pub fn restore_permissions(dir: &Path) {
    let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o755));
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_symlink() => {}
            Ok(kind) if kind.is_dir() => restore_permissions(&path),
            _ => {
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o644));
            }
        }
    }
}

/// Whether the check runs as the root user (permission cases cannot fail
/// for root, so they are skipped). Asks `id -u`, which needs no `unsafe`.
pub fn running_as_root() -> bool {
    Command::new("/usr/bin/id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}
