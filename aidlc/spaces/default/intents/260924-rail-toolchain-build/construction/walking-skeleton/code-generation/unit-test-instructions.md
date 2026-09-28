# Unit Test Instructions — walking-skeleton (U1)

## Test framework and configuration

- **Framework:** Rust's built-in test harness (`#[test]`), run with **cargo-nextest**, a tool installed in CI and locally, not a workspace dependency.
- **Per-test time limit:** `.config/nextest.toml` sets `slow-timeout = { period = "30s", terminate-after = 2 }` (60 seconds; performance design).
- **Test helper crate:** `crates/rail-testkit`, a workspace member used only as a `dev-dependency` and never shipped. It provides:
  - **Golden files:** `assert_golden(path, actual)` compares output with a committed expected file and rewrites it only when `RAIL_BLESS=1` is set.
  - **Process runner:** starts `rail` or a built program with a 60-second deadline and returns exit status, standard output and standard error.
  - **RAP client:** writes `Content-Length`-framed requests to a `rail rap` child and reads framed responses.
- **Coverage:** `cargo-llvm-cov` with nextest, Linux, floor 80% lines. It excludes `fuzz/`, `crates/rail-testkit`, test files and golden files.
- No third-party test crate is used (NFR Q4 A).

## How to run THIS unit's tests

The walking skeleton owns exactly these packages:

- `rail`
- `rail-json`
- `rail-diag`
- `rail-syntax`
- `rail-check`
- `rail-lower`
- `rail-codegen`
- `rail-build`
- `rail-tools`
- `rail-rap`
- `rail-testkit`

Run only them:

```bash
cargo nextest run --locked -p rail -p rail-json -p rail-diag -p rail-syntax -p rail-check -p rail-lower -p rail-codegen -p rail-build -p rail-tools -p rail-rap -p rail-testkit
```

This command becomes runnable at plan Step 3 (test-runner bootstrap). From then on, each Red step runs it and records the failing output before its Green step.

End-to-end tests only (the WF6 acceptance tests in `crates/rail/tests/e2e.rs`):

```bash
cargo nextest run --locked -p rail --test e2e
```

Coverage (Linux):

```bash
cargo llvm-cov nextest --locked -p rail -p rail-json -p rail-diag -p rail-syntax -p rail-check -p rail-lower -p rail-codegen -p rail-build -p rail-tools -p rail-rap --fail-under-lines 80
```

JSON parser fuzz target (nightly toolchain, `fuzz/` directory; runs 30 minutes in nightly CI):

```bash
cargo +nightly fuzz run json_parse -- -max_total_time=60
```

The 60-second value is for local smoke runs; CI uses `-max_total_time=1800`.

## Expected coverage

- **At least 80% line coverage** of the Rust code listed above, measured on Linux. The floor can go up, never down, and it is never lowered to make a build pass.
- **`rail-runtime`** is a `no_std` static library. It is compiled into the programs `rail` builds, not into the test process, so `cargo-llvm-cov` cannot measure its lines. Its behaviour (print, exit codes, traps) is tested through compiled programs in the end-to-end and `rail-build` tests. This measurement gap is stated here; the floor is not lowered for it.

## Test volume (standard strategy: 5–8 tests per component)

| Package | Tests (approx.) | Focus |
|---|---|---|
| `rail` (e2e) | 10–12 | WF6 steps 1–7, both fixtures, CLI and protocol, exit codes, determinism, a malformed message mid-session |
| `rail-json` | 8 + suite | Round trip; strict rejects (duplicate keys, trailing data, bad UTF-8); depth 64 / 65; JSONTestSuite `y_`/`n_` cases; seeded random-input test (no panic) |
| `rail-diag` | 5–6 | Diagnostic shape; sort order; rule registry (TY001, FX001, SKL001 only); SKL001 never redefined |
| `rail-syntax` | 8 | Accept fixtures; round trip byte-exact; SKL001 cases (unsupported form, layout, bad ID, duplicate ID, reserved `skel` module, nesting 257, 16 MiB + 1); located parse errors |
| `rail-check` | 8 | TY001 (mismatch, match arms, main shape, Caps/Result misuse); FX001 (missing `log`); SKL001 (unsupported type, suffix, shadowing, unused binding, missing signature/export) |
| `rail-lower` | 5 | Lowering of literals, calls, `let`, Boolean match, checked arithmetic |
| `rail-codegen` | 5 | Object file produced for the fixtures; deterministic bytes; trap branches present |
| `rail-build` | 6–7 | Build and run `skeleton.answer` (exit 0, prints 42); Err (exit 1); overflow and division by zero (exit 70); blocked build; `cc` missing; no partial artifact; two-directory byte-identical build; no absolute path in the binary |
| `rail-tools` | 6–7 | Path confinement (`..`, absolute, symlink out); panic boundary gives `internal.error`; ToolError codes; run time limit and output cap (helper executables); signal gives `run.failed` |
| `rail-rap` | 8 | Framing (good, missing length, 1 KiB header, 16 MiB + 1 discarded); `-32700`, `-32600`, `-32601`, `-32602`, `rap.unsupported_param`, `rap.not_initialized`; repeated `initialize`; notifications get no response; session continues after each error; stdout carries frames only |
| `rail-testkit` | 3 | Golden compare and bless switch; process-runner deadline |

## Mocking and stubbing

- No mocks of our own code. Tests use real crates end to end at their boundaries.
- **Process-level fakes:** three small helper executables built as `[[bin]]` targets of `rail-testkit`. That crate is test tooling that never ships.
  - `never-ends` sleeps forever (tests the run time limit).
  - `floods-output` writes without end (tests the output cap).
  - `aborts` aborts by signal.
  The run operation's tests start them through the same child-process code path `rail` uses for built programs.
- **`cc` missing:** the test runs `rail build` with `PATH` set to an empty temporary directory.
- **Panic boundary:** a test-only operation in `rail-tools`, compiled only under `cfg(test)`, panics on purpose. `rail-tools`' own unit tests exercise it.

## Test data

| Data | Location | Notes |
|---|---|---|
| Fixtures | `fixtures/skeleton/answer.rlc`, `fixtures/skeleton/broken.rlc` (workspace root `fixtures/`, so the qnames are `skeleton.answer` and `skeleton.broken`) | Hand-written canonical text with fixed IDs |
| Negative source cases | Built inside each test from small strings | One case per rule |
| Golden files | `crates/<crate>/tests/golden/` | Diagnostics, protocol responses, CLI output. Shared by both platforms, so a cross-platform difference fails |
| JSONTestSuite | `tests/data/JSONTestSuite/` | The `test_parsing` cases from `github.com/nst/JSONTestSuite` (MIT), with the upstream licence file and the upstream commit recorded |
| Random inputs | Generated in the test | A fixed seed (logged), pseudo-random bytes from an in-test generator (no crate) |

Every test owns its temporary directory, created under the system temporary directory and removed at the end. No test depends on another test's state or order.
