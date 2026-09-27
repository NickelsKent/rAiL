# Reliability Design — walking-skeleton (U1)

How the skeleton stays up, fails cleanly, leaves nothing half-written, and gives the same answer every time (`reliability-requirements.md`). It also records the quality gates from `tech-stack-decisions.md` that keep those properties true as the code grows.

## Sources

- `reliability-requirements.md`: NFR3.1–NFR3.3, NFR6.1, NFR9.6–NFR9.10.
- `tech-stack-decisions.md`: NFR6.2, NFR7.6, NFR9.1–NFR9.4.
- `functional-spec.md` and `rules.md` for this unit: error handling, BR4.3, BR5.3, BR5.5, BR6.4.
- `contract-summary.md`: E1 error codes, E2 exit codes, contract rule 6 (typed errors).
- `nfr-design-questions.md` [Q1] (build directory).

## Error taxonomy

Every failure ends as exactly one of the following. The table completes the functional design's ToolError list with the two additions from the reliability requirements.

| Kind | Code | CLI exit | Protocol |
|---|---|---|---|
| Problem in the user's program | diagnostics `TY001`, `FX001`, `SKL001` | 1 | Result with `blocking: true`; `build.blocked` for build and run |
| Module missing, unreadable or outside the workspace | `module.not_found`, `module.unreadable` | 3 | `-32000` + ToolError |
| No entry point | `build.no_entry` | 1 | `-32000` + ToolError |
| Code generation or link failure, `cc` missing | `build.failed` | 3 | `-32000` + ToolError |
| Program could not start, was killed at a limit, or ended by a signal | `run.failed` (message says which: "could not start", "time limit exceeded", "output limit exceeded", "ended by signal <NAME>") | 3 | `-32000` + ToolError |
| Unexpected internal failure | `internal.error` (added by NFR9.7) | 3 | `-32000` + ToolError |
| Protocol-level errors | — | — | `-32700`, `-32600`, `-32601`, `-32602`; `rap.not_initialized`, `rap.unsupported_param` as ToolErrors |
| Usage error (command line only) | — | 2 | — |

## Failure isolation (NFR9.6, NFR9.7)

- ToolServices runs each operation inside a panic boundary (`std::panic::catch_unwind`). A panic in any compiler crate becomes `internal.error`, with the panic message as the ToolError message. The `rail` binary builds with `panic = "unwind"` so this works; only the runtime library builds with `panic = "abort"`.
- The default panic hook is replaced so that a caught panic writes nothing to standard output. If logging is on, one `error` log line goes to standard error.
- The request loop holds no state an operation can corrupt. The session's two values are only written by `initialize`, so after a failed request the next request sees the same session.
- **Fault-injection test:** a test-only operation, compiled only under `cfg(test)`, panics on purpose. The test checks that the server answers `internal.error` and then answers a normal `check.run` correctly.

## No partial output (NFR9.8)

Build steps for module `a.b`, all under `.rail/build/dev/` [Q1]:

1. Write the object file to `a/b.o.partial`, then rename it to `a/b.o`.
2. Link with `cc` into `a/b.partial`.
3. On success, rename `a/b.partial` to `a/b`, which is atomic on the same file system. Then delete `a/b.o`.
4. On any failure, delete both `.partial` files and any `a/b.o`, and return `build.failed`. An executable from an earlier successful build is left untouched.

Only one `rail` process may build a given module at a time. Two concurrent builds of the same module from two processes are not supported in the skeleton: the last rename wins, and neither result is corrupt.

## Signals and `cc` (NFR9.9, NFR9.10)

- After a program run, the exit status is inspected. A normal exit returns its code (0, 1 or 70). Termination by a signal returns `run.failed` naming the signal (for example `SIGSEGV`), and is never mapped to an exit code.
- `cc` is found through `PATH`. If it is not found, the result is `build.failed` with "`cc` not found; install the Xcode Command Line Tools (macOS) or a C compiler (Linux)". If `cc` fails, the result is `build.failed` with its first 20 lines of standard error.

## Determinism (NFR3.1, NFR3.2, NFR3.3, NFR6.1)

| Source of variation | Control |
|---|---|
| Map iteration order | Ordered collections (`BTreeMap`, `Vec` sorted before output) wherever order is visible; `clippy::disallowed_types` denies `std::collections::HashMap`/`HashSet` in crates that produce output |
| JSON key order | The in-house writer writes fields in the order each result type declares them; nothing is sorted at run time |
| Diagnostics order | Sorted by (module, def, path, span, rule) (BR4.3) |
| Absolute paths | Results carry workspace-relative paths with `/`. `cc` runs in the build directory with relative arguments. The build emits no debug information in the skeleton |
| Timestamps, host names, random values | None are read by the toolchain. The linker's content-derived IDs (the Mach-O UUID, the ELF build ID) are functions of the inputs, so they are stable |
| Environment | Built programs run with a cleared environment |

**Verification design:**

- **Twice, in two directories.** `skeleton.answer` is built in two different temporary directories, and the executables must be byte-identical.
- **No leaked paths.** The executable is scanned for the bytes of the workspace's absolute path, which must not be present.
- **Across platforms.** Check and protocol results for both fixtures are recorded as golden files. The same golden files are used on macOS and Linux, so any cross-platform difference fails a test (NFR6.1).

## Quality gates (NFR6.2, NFR9.1–NFR9.4)

| Gate | Design |
|---|---|
| Both platforms green before merge (NFR6.2) | Branch protection on `main` requires the `test (macos)` and `test (linux)` checks |
| Coverage ≥ 80% on Linux (NFR9.1) | `cargo llvm-cov nextest --locked --fail-under-lines 80`, excluding `fuzz/`, test helpers and golden files |
| Tests first (NFR9.2) | The end-to-end check (WF6) is added first and fails; then each crate's tests are added before its code. Reviewers check the order in the pull request's commits |
| Formatting and lints (NFR9.3) | `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings` on both platforms |
| Bless step (NFR9.4) | The test helper rewrites an expected file only when `RAIL_BLESS=1`. CI never sets it, and fails if a test would change a golden file |
| Runtime dependencies (NFR7.6) | After building `skeleton.answer`, CI runs `otool -L` (macOS) or `ldd` (Linux) and compares the result with an allow-list: `libSystem` on macOS; `libc`, `ld-linux` and `linux-vdso` on Linux |
