//! The four skeleton operations (parse, check, build, run) over workspace access.
//!
//! Building block: ToolServices (contract C4). Owning unit: U1 walking-skeleton
//! (thin first version; completed by U5 agent-loop).

#![forbid(unsafe_code)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

mod error;
mod log;
mod ops;
mod run;
mod tree;
mod workspace;

pub use error::{ErrorCode, ToolError};
pub use log::{LogLevel, Record, invoke_logged, perform_logged};
pub use ops::{
    Operation, Outcome, ParseOutcome, artifact_to_json, build, check, guarded,
    install_quiet_panic_hook, invoke, parse, perform, run,
};
pub use run::{RunLimits, RunResult, run_executable};
pub use workspace::{MAX_SOURCE_BYTES, ModulePath, ModuleSource, Workspace};

#[cfg(test)]
mod tests {
    use super::*;

    /// A test-only operation that fails by panicking (NFR9.7).
    fn panicking_operation() -> Result<(), ToolError> {
        panic!("injected fault");
    }

    #[test]
    fn a_panic_becomes_internal_error() {
        let err = guarded(panicking_operation).unwrap_err();
        assert_eq!(err.code, ErrorCode::InternalError);
        assert_eq!(err.code.as_str(), "internal.error");
        assert_eq!(err.message, "internal error: injected fault");
        assert_eq!(err.code.exit_code(), 3);
    }

    #[test]
    fn the_boundary_passes_results_through() {
        assert_eq!(guarded(|| Ok(7)), Ok(7));
        let err = ToolError::new(ErrorCode::BuildFailed, "x");
        assert_eq!(guarded(|| Err::<(), _>(err.clone())), Err(err));
    }
}
