# Observability Requirements — walking-skeleton (U1)

The skeleton runs on the user's machine and in CI. There are no servers to monitor, so there are no metrics pipelines, dashboards, traces or alerts. Observability means: when something goes wrong, the person or agent running `rail` can see **what failed and where**, from the result itself and from standard error, without the logs ever touching standard output or making output non-deterministic.

## Sources

- `requirements.md`: FR13.2 (standard output carries only protocol messages), NFR3 (determinism).
- Team practice (Code Style): library code never prints to standard output; output never depends on clocks.
- `functional-spec.md` for this unit: error handling, BR6.5.
- `reliability-requirements.md` for this unit: NFR9.6–NFR9.10.

## Requirements

| ID | Requirement | Verification |
|---|---|---|
| NFR9.11 | Every failure is visible in the result itself: a diagnostic with a location, or a ToolError with a stable code and a message. Logs are never the only place a failure is reported. | Tests assert on results, never on log text |
| NFR9.12 | Logging goes to standard error only. Library crates never write to standard output or standard error directly; they report through return values, and only the `rail` binary writes to the streams. | clippy `print_stdout` / `print_stderr` denied in library crates |
| NFR9.13 | Logging is off by default. Setting the environment variable `RAIL_LOG` to `error`, `info` or `debug` turns on one-line log records on standard error. Each record carries the level, the operation, the module, and for errors the ToolError code. | Test that default runs write nothing to standard error; a test with `RAIL_LOG=debug` |
| NFR3.5 | Log records contain no timestamps, absolute paths, hostnames or process IDs, so logs from two runs can be compared directly. Paths are workspace-relative. | Test comparing two `RAIL_LOG=debug` runs |
| NFR9.14 | The end-to-end verification (WF6) prints a short, readable pass/fail line for each step and, on failure, the step's command and captured output. That gives CI logs enough to diagnose a failure without re-running locally. | Review of the verification command's output on a forced failure |

## Not applicable to the skeleton

- Metrics, dashboards, distributed tracing and alerting: nothing is deployed.
- Service-level indicators and objectives: there is no service.
- Audit logging of capability use: U15 (FR37).
- Profiling: U7 (FR26.3).
