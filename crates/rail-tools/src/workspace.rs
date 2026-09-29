//! Workspace access: path confinement and bounded reads (NFR5.3, NFR4.4).

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use rail_diag::{Diagnostic, Location, SKL001, Span};

use crate::{ErrorCode, ToolError};

/// The largest source file the skeleton reads (16 MiB).
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;

/// A workspace rooted at a canonical directory.
#[derive(Clone, Debug)]
pub struct Workspace {
    root: PathBuf,
}

/// A validated module path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModulePath {
    /// Workspace-relative path with `/`, as given (e.g. `skeleton/answer.rlc`).
    pub rel: String,
    /// The module qname derived from the path (e.g. `skeleton.answer`).
    pub qname: String,
    /// Canonical absolute path, inside the workspace root. Never reported.
    pub abs: PathBuf,
}

/// A module's bytes, read with the size bound.
#[derive(Clone, Debug)]
pub struct ModuleSource {
    /// Where it came from.
    pub path: ModulePath,
    /// At most [`MAX_SOURCE_BYTES`] + 1 bytes.
    pub bytes: Vec<u8>,
    /// Whether the file is larger than [`MAX_SOURCE_BYTES`].
    pub oversized: bool,
}

impl ModuleSource {
    /// `SKL001` at byte 16 MiB when the file is over the limit.
    pub fn limit_diagnostic(&self) -> Option<Diagnostic> {
        self.oversized.then(|| {
            Diagnostic::error(
                SKL001,
                Location {
                    module: self.path.qname.clone(),
                    def: "#000000".to_string(),
                    path: String::new(),
                    span: Span::new(MAX_SOURCE_BYTES, MAX_SOURCE_BYTES + 1),
                },
                "source file is larger than 16 MiB",
            )
        })
    }
}

impl Workspace {
    /// Opens the workspace at `root`, canonicalised once.
    pub fn open(root: &Path) -> Result<Workspace, ToolError> {
        let root = root.canonicalize().map_err(|e| {
            ToolError::new(
                ErrorCode::ModuleNotFound,
                format!("workspace root cannot be opened: {}", e.kind()),
            )
        })?;
        if !root.is_dir() {
            return Err(ToolError::new(
                ErrorCode::ModuleNotFound,
                "workspace root is not a directory",
            ));
        }
        Ok(Workspace { root })
    }

    /// The canonical root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Validates `module` and confines it to the root (NFR5.3).
    pub fn resolve(&self, module: &str) -> Result<ModulePath, ToolError> {
        let not_found = || {
            ToolError::new(
                ErrorCode::ModuleNotFound,
                format!("module {} not found in the workspace", printable(module)),
            )
        };
        let qname = qname_of(module).ok_or_else(not_found)?;
        let abs = self.root.join(module).canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                unreadable(module)
            } else {
                not_found()
            }
        })?;
        if !abs.starts_with(&self.root) {
            return Err(not_found());
        }
        Ok(ModulePath {
            rel: module.to_string(),
            qname,
            abs,
        })
    }

    /// Resolves and reads `module`, reading at most 16 MiB + 1 bytes.
    pub fn read_module(&self, module: &str) -> Result<ModuleSource, ToolError> {
        let path = self.resolve(module)?;
        let file = File::open(&path.abs).map_err(|_| unreadable(module))?;
        let limit = u64::try_from(MAX_SOURCE_BYTES).unwrap_or(u64::MAX) + 1;
        let mut bytes = Vec::new();
        file.take(limit)
            .read_to_end(&mut bytes)
            .map_err(|_| unreadable(module))?;
        let oversized = bytes.len() > MAX_SOURCE_BYTES;
        Ok(ModuleSource {
            path,
            bytes,
            oversized,
        })
    }
}

fn unreadable(module: &str) -> ToolError {
    ToolError::new(
        ErrorCode::ModuleUnreadable,
        format!("module {} cannot be read", printable(module)),
    )
}

/// The path as it may appear in a message: control characters escaped.
fn printable(module: &str) -> String {
    module.escape_default().to_string()
}

/// The qname of a well-formed module path: lowercase segments separated by
/// `/`, ending in `.rlc`. Rejects empty, absolute, `.`/`..`, backslash and
/// NUL paths by construction.
fn qname_of(module: &str) -> Option<String> {
    let stem = module.strip_suffix(".rlc")?;
    let segments: Vec<&str> = stem.split('/').collect();
    let valid = |s: &&str| {
        let mut bytes = s.bytes();
        bytes.next().is_some_and(|b| b.is_ascii_lowercase())
            && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    };
    segments.iter().all(valid).then(|| segments.join("."))
}
