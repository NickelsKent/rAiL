# Observability Design — walking-skeleton (U1)

Everything that goes wrong shows up in the result itself. Optional log lines on standard error add context when you ask for them, and the end-to-end check prints enough for CI logs to explain a failure on their own (`observability-requirements.md`).

## Sources

- `observability-requirements.md`: NFR3.5, NFR9.11–NFR9.14.
- `reliability-design.md` for this unit: error taxonomy.
- `functional-spec.md` for this unit: WF6 (end-to-end demonstration), BR6.5.

## Results carry the failure (NFR9.11)

Every failure path in `reliability-design.md` ends in a diagnostic or a ToolError with a stable code and a message. Tests assert on those results, never on log text. Logs are extra context only.

## Logging (NFR9.12, NFR9.13, NFR3.5)

- **Who writes.** Only the `rail` binary writes to the streams. Library crates return values; they never print. `clippy::print_stdout` and `clippy::print_stderr` are denied in every library crate. The binary logs at operation boundaries: start and end at `debug`, and failures at `error` with their ToolError code.
- **Switch.** Logging is off by default. `RAIL_LOG=error|info|debug` turns it on. Any other value is treated as off, with one warning line on standard error.
- **Format.** One line per record, `rail <level> <operation> <module> <message>`, with `code=<ToolError code>` added for failures, for example:

  ```
  rail error build.run skeleton/answer.rlc code=build.failed cc not found
  ```

  Records contain no timestamps, process IDs, host names or absolute paths. Module paths are workspace-relative. Two runs therefore produce identical logs.
- **Where.** Standard error only, including in `rail rap` mode, where standard output is reserved for protocol frames.

## End-to-end check output (NFR9.14)

The recorded verification command (chosen at the skeleton checkpoint) runs the WF6 steps and prints one line per step:

```
PASS 1 check skeleton.answer
FAIL 4 run skeleton.broken: expected build.blocked with TY001, got exit 0
  command: rail run --json skeleton/broken.rlc
  stdout: ...
  stderr: ...
```

On a failure it prints the step's command and its captured output, so the CI log shows what happened without a local re-run. It exits non-zero if any step fails.

## Deliberately absent

- Metrics, tracing, dashboards and alerts: nothing is deployed.
- Audit log of capability use: U15.
- Profiles: U7.
