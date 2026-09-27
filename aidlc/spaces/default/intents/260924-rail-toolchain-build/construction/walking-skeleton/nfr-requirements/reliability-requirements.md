# Reliability Requirements — walking-skeleton (U1)

There is no running service, so availability percentages and recovery objectives do not apply. For the skeleton, reliability means four things:

- One bad request never takes down the protocol server.
- Every failure ends as a typed result, never a crash.
- Nothing is left half-written.
- The same input gives the same result on every run and both platforms.

## Sources

- `requirements.md`: FR13.1 (a failing request never ends the server), NFR3 (determinism), NFR6 (portability), FR27.
- `functional-spec.md` and `rules.md` for this unit: error handling, BR4.3, BR5.5, BR6.4, WF6.
- `contract-summary.md`: contract rule 6 (errors are typed values; nothing panics across a boundary).
- Team practice: one bad request never crashes the agent protocol server; output never depends on hash-map order, clocks or randomness.

## Requirements

| ID | Requirement | Verification |
|---|---|---|
| NFR3.1 | Parse, check and protocol results are identical for identical input, on every run, in every directory, and on both platforms. Diagnostics are sorted (BR4.3), and JSON objects are written with a fixed key order. | Run each fixture twice and in two directories; compare bytes; compare across the two CI platforms |
| NFR3.2 | Builds are byte-identical for the same module, platform and toolchain version. No timestamps, absolute paths, hostnames or random seeds are written into objects or executables (BR5.5, firm rule). | Build twice in different directories; compare bytes; scan the binary for the workspace path |
| NFR3.3 | Toolchain output never depends on hash-map iteration order, the clock or randomness. Ordered maps or sorted output are used wherever order is visible. | clippy `disallowed_types` for `HashMap`/`HashSet` in output-producing crates, or a review check |
| NFR6.1 | Observable results are identical on macOS (AArch64) and Linux (x86-64): diagnostics, protocol responses, command exit codes, and the skeleton program's output and exit code. All paths in results are workspace-relative with `/` separators. | WF6 run on both CI platforms with a cross-platform comparison of recorded outputs |
| NFR9.6 | A failing or malformed protocol request returns a structured error and the server keeps serving. The same session answers the next valid request correctly (BR6.4). | Protocol tests: bad framing, bad JSON, unknown method, bad params, tool failure, each followed by a valid request |
| NFR9.7 | No panic crosses an operation boundary. An unexpected internal failure inside one operation is caught at the ToolServices boundary and reported as a tool failure: exit code 3 on the command line, `-32000` over the protocol, with the ToolError code `internal.error` in both cases. This requirement adds `internal.error` to the functional design's list of ToolError codes. The server keeps running. | Fault-injection test with a test-only operation that panics |
| NFR9.8 | A failed build leaves no artifact behind. The executable is written to a temporary file in the build directory and renamed into place only when linking succeeded. Temporary object files are removed on success and on failure. | Test: force a link failure; check that the build directory contains no artifact or temporary file |
| NFR9.9 | A built program that ends by a signal (for example a stack overflow, which the skeleton does not yet turn into a trap) is reported as `run.failed` naming the signal. It is never reported as a normal exit code. | Test with a test-only helper that aborts by signal |
| NFR9.10 | If the `cc` driver is missing or fails, the build fails with `build.failed`, and the message says `cc` was not found or shows its first error lines. It never panics. | Test with `PATH` set so that `cc` is not found |

## Not applicable to the skeleton

- Availability, SLOs, RTO and RPO: there is no running service.
- Retries: every operation is local and deterministic, so a retry gives the same result. Nothing is retried automatically.
- Backups: the skeleton owns no data. The workspace is the user's own files under version control.
