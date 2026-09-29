//! Log records for `RAIL_LOG` (NFR9.12, NFR9.13, NFR3.5). Library code only
//! builds records; the `rail` binary decides whether to write them to
//! standard error.

use rail_json::Value;

use crate::ops::{Outcome, perform};
use crate::{Operation, ToolError, Workspace};

/// A log level; a configured level includes every level before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Failures.
    Error,
    /// Operation outcomes.
    Info,
    /// Operation starts.
    Debug,
}

impl LogLevel {
    /// The level's name.
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Error => "error",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
        }
    }

    /// The level named by a `RAIL_LOG` value.
    pub fn parse(value: &str) -> Option<LogLevel> {
        [LogLevel::Error, LogLevel::Info, LogLevel::Debug]
            .into_iter()
            .find(|l| l.as_str() == value)
    }
}

/// One log record. It holds no timestamp, process ID, host name or absolute
/// path, so two runs log identical lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// Level.
    pub level: LogLevel,
    /// Operation (a protocol method name, or `rap`).
    pub operation: String,
    /// The module path as the caller gave it (workspace-relative), or `-`.
    pub module: String,
    /// The ToolError code, for failures.
    pub code: Option<&'static str>,
    /// Message.
    pub message: String,
}

impl Record {
    /// `rail <level> <operation> <module> [code=<code>] <message>`.
    pub fn line(&self) -> String {
        let code = self.code.map(|c| format!(" code={c}")).unwrap_or_default();
        format!(
            "rail {} {} {}{code} {}",
            self.level.as_str(),
            self.operation,
            self.module,
            self.message
        )
    }
}

/// Runs `op` like [`crate::perform`], logging its start, outcome or failure.
pub fn perform_logged(
    ws: &Workspace,
    op: Operation,
    module: &str,
    log: &mut dyn FnMut(Record),
) -> Result<Outcome, ToolError> {
    let record = |level, code, message: &str| Record {
        level,
        operation: op.method().to_string(),
        module: module.to_string(),
        code,
        message: message.to_string(),
    };
    log(record(LogLevel::Debug, None, "start"));
    let result = perform(ws, op, module);
    match &result {
        Ok(_) => log(record(LogLevel::Info, None, "ok")),
        Err(e) => log(record(LogLevel::Error, Some(e.code.as_str()), &e.message)),
    }
    result
}

/// [`perform_logged`], returning the JSON result.
pub fn invoke_logged(
    ws: &Workspace,
    op: Operation,
    module: &str,
    log: &mut dyn FnMut(Record),
) -> Result<Value, ToolError> {
    perform_logged(ws, op, module, log).map(|o| o.to_json())
}
