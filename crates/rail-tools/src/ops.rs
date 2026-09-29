//! The four operations shared by the command line and the protocol (BR6.1),
//! each behind a panic boundary (NFR9.7).

use std::panic::{AssertUnwindSafe, catch_unwind};

use rail_build::{Artifact, BuildConfig, BuildError};
use rail_diag::{CheckResult, Diagnostic};
use rail_json::Value;
use rail_syntax::Module;

use crate::run::{RunLimits, RunResult, run_executable};
use crate::tree::tree_to_json;
use crate::{ErrorCode, ToolError, Workspace};

/// One of the skeleton's operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// `rail parse` / `tree.get`
    Parse,
    /// `rail check` / `check.run`
    Check,
    /// `rail build` / `build.run`
    Build,
    /// `rail run` / `run.run`
    Run,
}

impl Operation {
    /// All operations, in capability order.
    pub const ALL: [Operation; 4] = [
        Operation::Parse,
        Operation::Check,
        Operation::Build,
        Operation::Run,
    ];

    /// The command-line command.
    pub fn command(self) -> &'static str {
        match self {
            Operation::Parse => "parse",
            Operation::Check => "check",
            Operation::Build => "build",
            Operation::Run => "run",
        }
    }

    /// The protocol method.
    pub fn method(self) -> &'static str {
        match self {
            Operation::Parse => "tree.get",
            Operation::Check => "check.run",
            Operation::Build => "build.run",
            Operation::Run => "run.run",
        }
    }

    /// The operation for a command-line command.
    pub fn from_command(command: &str) -> Option<Operation> {
        Operation::ALL
            .into_iter()
            .find(|op| op.command() == command)
    }

    /// The operation for a protocol method.
    pub fn from_method(method: &str) -> Option<Operation> {
        Operation::ALL.into_iter().find(|op| op.method() == method)
    }
}

/// The result of `parse`: the tree, or the diagnostics that stopped it.
#[derive(Clone, Debug)]
pub struct ParseOutcome {
    /// Module qname.
    pub module: String,
    /// The tree when parsing succeeded.
    pub tree: Option<Module>,
    /// The `SKL001` diagnostic when it did not.
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseOutcome {
    /// `{module, tree, diagnostics}`.
    pub fn to_json(&self) -> Value {
        Value::object([
            ("module", Value::str(&self.module)),
            ("tree", self.tree.as_ref().map_or(Value::Null, tree_to_json)),
            (
                "diagnostics",
                Value::Array(self.diagnostics.iter().map(Diagnostic::to_json).collect()),
            ),
        ])
    }
}

/// Parses a module.
pub fn parse(ws: &Workspace, module: &str) -> Result<ParseOutcome, ToolError> {
    let source = ws.read_module(module)?;
    let qname = source.path.qname.clone();
    if let Some(limit) = source.limit_diagnostic() {
        return Ok(ParseOutcome {
            module: qname,
            tree: None,
            diagnostics: vec![limit],
        });
    }
    Ok(match rail_syntax::parse(&qname, &source.bytes) {
        Ok(tree) => ParseOutcome {
            module: qname,
            tree: Some(tree),
            diagnostics: Vec::new(),
        },
        Err(d) => ParseOutcome {
            module: qname,
            tree: None,
            diagnostics: vec![*d],
        },
    })
}

/// Checks a module.
pub fn check(ws: &Workspace, module: &str) -> Result<CheckResult, ToolError> {
    let source = ws.read_module(module)?;
    if let Some(limit) = source.limit_diagnostic() {
        return Ok(CheckResult::new(source.path.qname, vec![limit]));
    }
    Ok(rail_check::check(&source.path.qname, &source.bytes).result)
}

fn blocked(result: &CheckResult) -> ToolError {
    ToolError {
        code: ErrorCode::BuildBlocked,
        message: format!("{} has blocking diagnostics", result.module),
        data: Some(result.to_json()),
    }
}

/// Builds a module (dev mode, host target).
pub fn build(ws: &Workspace, module: &str) -> Result<Artifact, ToolError> {
    let source = ws.read_module(module)?;
    let qname = source.path.qname.clone();
    if let Some(limit) = source.limit_diagnostic() {
        return Err(blocked(&CheckResult::new(qname, vec![limit])));
    }
    rail_build::build(ws.root(), &qname, &source.bytes, &BuildConfig::default()).map_err(
        |e| match e {
            BuildError::Blocked(result) => blocked(&result),
            BuildError::NoEntry => ToolError::new(
                ErrorCode::BuildNoEntry,
                format!("{qname} has no exported main function"),
            ),
            BuildError::Failed(message) => ToolError::new(ErrorCode::BuildFailed, message),
        },
    )
}

/// Builds and runs a module.
pub fn run(ws: &Workspace, module: &str) -> Result<RunResult, ToolError> {
    let artifact = build(ws, module)?;
    run_executable(&ws.root().join(&artifact.path), &RunLimits::default())
}

/// `{module, target, mode, path}` of a build.
pub fn artifact_to_json(artifact: &Artifact) -> Value {
    Value::object([
        ("module", Value::str(&artifact.module)),
        ("target", Value::str(artifact.target)),
        ("mode", Value::str(artifact.mode)),
        ("path", Value::str(&artifact.path)),
    ])
}

/// The typed result of one operation.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// From `parse`.
    Parsed(ParseOutcome),
    /// From `check`.
    Checked(CheckResult),
    /// From `build`.
    Built(Artifact),
    /// From `run`.
    Ran(RunResult),
}

impl Outcome {
    /// The JSON result: the one shape both `--json` and the protocol use
    /// (BR6.1).
    pub fn to_json(&self) -> Value {
        match self {
            Outcome::Parsed(r) => r.to_json(),
            Outcome::Checked(r) => r.to_json(),
            Outcome::Built(a) => artifact_to_json(a),
            Outcome::Ran(r) => r.to_json(),
        }
    }
}

/// Runs `op` on `module` inside the panic boundary.
pub fn perform(ws: &Workspace, op: Operation, module: &str) -> Result<Outcome, ToolError> {
    guarded(|| match op {
        Operation::Parse => parse(ws, module).map(Outcome::Parsed),
        Operation::Check => check(ws, module).map(Outcome::Checked),
        Operation::Build => build(ws, module).map(Outcome::Built),
        Operation::Run => run(ws, module).map(Outcome::Ran),
    })
}

/// Runs `op` on `module` inside the panic boundary and returns its JSON
/// result.
pub fn invoke(ws: &Workspace, op: Operation, module: &str) -> Result<Value, ToolError> {
    perform(ws, op, module).map(|o| o.to_json())
}

/// Runs `operation`, turning a panic into `internal.error` (NFR9.7).
pub fn guarded<T>(operation: impl FnOnce() -> Result<T, ToolError>) -> Result<T, ToolError> {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or_else(|payload| {
        let detail = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_string());
        Err(ToolError::new(
            ErrorCode::InternalError,
            format!("internal error: {detail}"),
        ))
    })
}

/// Replaces the panic hook so a caught panic writes nothing to the streams;
/// the failure is reported through its `internal.error` result instead.
pub fn install_quiet_panic_hook() {
    std::panic::set_hook(Box::new(|_| {}));
}
