//! Typed tool failures (contract C4 `ToolError`).

use rail_json::Value;

/// A stable ToolError code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    /// The module path is invalid, missing, or outside the workspace.
    ModuleNotFound,
    /// The module exists but cannot be read.
    ModuleUnreadable,
    /// The module has blocking diagnostics; `data` holds the CheckResult.
    BuildBlocked,
    /// The module has no exported `main`.
    BuildNoEntry,
    /// Code generation or linking failed, or `cc` is missing.
    BuildFailed,
    /// The program could not start, hit a limit, or ended by a signal.
    RunFailed,
    /// A protocol request arrived before `initialize`.
    RapNotInitialized,
    /// A well-formed parameter the skeleton does not support.
    RapUnsupportedParam,
    /// An unexpected internal failure (NFR9.7).
    InternalError,
}

impl ErrorCode {
    /// The code's stable text.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::ModuleNotFound => "module.not_found",
            ErrorCode::ModuleUnreadable => "module.unreadable",
            ErrorCode::BuildBlocked => "build.blocked",
            ErrorCode::BuildNoEntry => "build.no_entry",
            ErrorCode::BuildFailed => "build.failed",
            ErrorCode::RunFailed => "run.failed",
            ErrorCode::RapNotInitialized => "rap.not_initialized",
            ErrorCode::RapUnsupportedParam => "rap.unsupported_param",
            ErrorCode::InternalError => "internal.error",
        }
    }

    /// The command-line exit code for this failure (E2): 1 when the user's
    /// program is the problem, 3 for a tool failure.
    pub fn exit_code(self) -> i32 {
        match self {
            ErrorCode::BuildBlocked | ErrorCode::BuildNoEntry => 1,
            _ => 3,
        }
    }
}

/// A structured failure of a tool operation; never a crash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolError {
    /// Stable code.
    pub code: ErrorCode,
    /// Human-readable message.
    pub message: String,
    /// Operation-specific data (the CheckResult for `build.blocked`).
    pub data: Option<Value>,
}

impl ToolError {
    /// A failure without data.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> ToolError {
        ToolError {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// `{code, message, data?}`.
    pub fn to_json(&self) -> Value {
        let mut fields = vec![
            ("code", Value::str(self.code.as_str())),
            ("message", Value::str(&self.message)),
        ];
        if let Some(data) = &self.data {
            fields.push(("data", data.clone()));
        }
        Value::object(fields)
    }
}
