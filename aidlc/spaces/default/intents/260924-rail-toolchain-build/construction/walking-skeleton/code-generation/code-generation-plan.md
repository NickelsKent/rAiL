# Code Generation Plan — walking-skeleton (U1)

This plan builds the walking skeleton: one Cargo workspace producing one `rail` binary. The binary parses, checks, serves over the agent protocol, builds with Cranelift, links with `cc`, and runs the two fixture modules on macOS and Linux. It follows the approved functional design, NFR requirements and NFR design for this unit. Tests come first (TDD): the end-to-end acceptance tests are written and fail before anything else, then each layer goes Red, Green, Refactor.

## Sources

- Functional design: `construction/walking-skeleton/functional-design/` (`functional-spec.md` WF1–WF6, `rules.md` BR1.1–BR6.6, `entities.md`).
- NFR requirements: `construction/walking-skeleton/nfr-requirements/` (NFR1.1–NFR10.2, `tech-stack-decisions.md`).
- NFR design: `construction/walking-skeleton/nfr-design/` (security, performance, scalability, reliability, observability, logical components).
- Contracts: `inception/contract-design/contract-summary.md` (C1–C4, E1, E2, E7). Unit: `inception/units-generation/unit-of-work.md` (U1). Requirement: FR27.
- There are no user stories in this plan (the User Stories stage was skipped). Each step traces to FR27 and to the business rules (BRx.y) and NFR requirements (NFRx.y) it implements.

## Prerequisites (before Step 1)

These run after this plan is approved and before any code is written. They follow your answers of 2026-09-27.

- [x] **P1. Planning records to `main`.** Commit the current planning and design records on `rail-toolchain-ideation` and open a pull request into `main` on `NickelsKent/rAiL`. You review and merge it. Code generation waits for the merge.
- [x] **P2. Bolt branch.** After the merge, update `main` and create `bolt/walking-skeleton` from it. All skeleton work goes on that branch and reaches `main` through one squash-merged pull request with green CI, the AI review and your approval.
- [x] **P3. Rust.** Install Rust with the official installer from `https://rustup.rs` (it installs into `~/.rustup` and `~/.cargo`). Then install:
  - the newest stable toolchain, with the `rustfmt`, `clippy` and `llvm-tools-preview` components;
  - the newest nightly (fuzzing only);
  - the tools `cargo-nextest`, `cargo-llvm-cov`, `cargo-deny` and `cargo-fuzz`.

  Record the exact stable and nightly versions.

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "Tests come first everywhere: at the start of each phase we write that phase's acceptance-suite harness so that it runs and fails, and for each unit we write its tests so that they fail before we write the code they check, then write the code until they pass.",
  "scope": "rail-toolchain",
  "test_strategy": "standard",
  "project_type": "greenfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "Tests are part of every Bolt. The work is not finished until its tests are\nwritten first and pass.\n\n- **Methodology**: tdd\n- **Ordering**: Tests come first everywhere: at the start of each phase we write that phase's acceptance-suite harness so that it runs and fails, and for each unit we write its tests so that they fail before we write the code they check, then write the code until they pass.\n\nAdditional notes (these do not replace the two fields above):\n\n- The spec's acceptance suites (T1–T14) are the top-level proof of\n  correctness. Each phase's exit criteria come from the approved scope\n  document, not from the raw spec roadmap, because the scope document changed\n  several exits (for example, the first usable slice exits when T1, T2, T4, T5\n  and T6 pass at CI volume).\n- Everyday CI runs the acceptance suites at scaled-down volumes, and the spec's\n  full volumes run as the release check. Each suite's CI volume and seed policy\n  is recorded in the repository. Reducing a recorded CI volume counts as\n  lowering a pass criterion.\n- Coverage: at least **80% line coverage** for the Rust code, measured on\n  Linux, required before merge. The benchmark corpus, the test harnesses and\n  code generated from the grammar are excluded. The floor can go up but never\n  down. rAiL code (such as the standard library) has no coverage gate; its\n  quality rests on the acceptance suites.\n- CI runs at three levels:\n  - **Every pull request:** a quick check on GitHub-hosted macOS and Linux\n    runners.\n  - **Nightly:** larger volumes and short fuzz runs.\n  - **Release check:** full volumes, on the owner's own machines.\n- Performance numbers (T13) count only when they come from the owner's\n  reference machines. Results from hosted CI runners are never T13 evidence.\n- Every defect fix comes with a test that reproduces the defect.\n- Coverage floors, acceptance-suite pass criteria and recorded CI volumes are\n  never lowered to make a build pass."
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:b9183e58b54fa6604be6ddd107184202051b33ba4f1c00854e5d1855278b3767",
  "contract_sha256": "sha256:b8ec90f0e1a2d843dec5f3f2a9212be14ca6a78cba2c41a0b0386c3c9bb1fb25"
}
```

## How the contract's layers map to this unit

| Contract layer | In the skeleton |
|---|---|
| Data model | `rail-json` (JSON values, reader, writer) and `rail-diag` (diagnostic shape, rule registry) |
| Repository / data access | `rail-tools` workspace access: path confinement and bounded file reads |
| Business logic | `rail-syntax`, `rail-check`, `rail-lower`, `rail-codegen`, `rail-build`, `rail-runtime`, and the `rail-tools` operations |
| API / endpoint | `rail-rap` (protocol server) and `rail` (command line) |
| Frontend | Not applicable: the skeleton has no user interface |

Each Red step runs the unit-scoped test command from `unit-test-instructions.md` and records the failing output in the step's notes before its Green step starts.

## Plan

### Step 1 — Project structure and production configuration skeleton

- [x] Create the Cargo workspace at the repository root:
  - `Cargo.toml` lists the workspace members `crates/rail`, `crates/rail-json`, `crates/rail-diag`, `crates/rail-syntax`, `crates/rail-check`, `crates/rail-lower`, `crates/rail-codegen`, `crates/rail-build`, `crates/rail-tools`, `crates/rail-rap` and `crates/rail-testkit`, with `resolver = "3"` and edition 2024. `crates/rail-runtime` is not a member: `rail-build` compiles it (Step 12).
  - `license = "MIT OR Apache-2.0"`.
  - `[profile.*]` sets `panic = "unwind"` for the compiler.
- [x] Add these root files:
  - `rust-toolchain.toml`: the stable version recorded in P3, with the `rustfmt`, `clippy` and `llvm-tools-preview` components.
  - `.gitignore`: add `/target/` and `/.rail/`.
  - `LICENSE-MIT` and `LICENSE-APACHE`.
- [x] Add Cranelift, pinned to one exact version (the newest release on the day), as `rail-codegen` dependencies: `cranelift-codegen`, `cranelift-frontend`, `cranelift-module` and `cranelift-object` (NFR7.4). No other third-party crate.
- [x] Put `#![forbid(unsafe_code)]` at the top of every crate root except `rail-codegen` (and `rail-runtime`), plus `#![deny(clippy::print_stdout, clippy::print_stderr)]` in every library crate (NFR4.1, NFR9.12).
- [x] Add `clippy.toml` with `disallowed-types` for `std::collections::HashMap` and `std::collections::HashSet` (NFR3.3).
- Traces: FR27, NFR4.1, NFR7.4, NFR3.3, NFR9.12.

### Step 2 — Acceptance harness first: the end-to-end tests (WF6), failing

- [x] Add the two fixtures:
  - `fixtures/skeleton/answer.rlc`: the happy path from `functional-spec.md` (canonical text, fixed IDs, every function exported).
  - `fixtures/skeleton/broken.rlc`: the same module with `answer`'s body `(== x 7)`.
- [x] Write `crates/rail/tests/e2e.rs`, one test per WF6 step:
  - `check` answer: no diagnostics.
  - `run` answer: stdout `42\n`, exit 0.
  - A protocol session on answer: `initialize`, `tree.get`, `check.run`, `build.run`, `run.run` return `{exit_code: 0, stdout: "42\n", stderr: ""}`.
  - `check` and `run` on broken, through the command line and the protocol: one `TY001` at `answer`'s body; the build is refused.
  - The server answers correctly after a malformed message.
  - Two builds are byte-identical.
  - Outputs match the shared golden files.
- [x] Traces: FR27, BR6.1, NFR6.1. These tests stay failing until Step 16. They are the unit's acceptance proof.

### Step 3 — Bootstrap the test runner and record the unit-scoped command

- [x] Add `.config/nextest.toml` with `slow-timeout = { period = "30s", terminate-after = 2 }` (NFR1.2).
- [x] Create `crates/rail-testkit`:
  - golden-file compare with the `RAIL_BLESS=1` rewrite switch (NFR9.4);
  - a process runner with a 60-second deadline;
  - a framed RAP client;
  - helper binaries `never-ends`, `floods-output` and `aborts`.
- [x] Run the unit-scoped command from `unit-test-instructions.md`. It must compile and run, with the Step 2 tests failing. Record its output.
- Traces: NFR1.2, NFR9.2, NFR9.4.

### Step 4 — Data model, Red: `rail-json` and `rail-diag` tests

- [x] Vendor the JSONTestSuite `test_parsing` cases from `https://github.com/nst/JSONTestSuite` into `tests/data/JSONTestSuite/`, with its MIT licence file and the upstream commit hash recorded in `tests/data/JSONTestSuite/SOURCE.md` (NFR4.3).
- [x] `rail-json` tests:
  - round trip;
  - every `y_` case accepted and every `n_` case rejected, with no panic;
  - duplicate keys, trailing data and bad UTF-8 rejected;
  - depth 64 accepted and 65 rejected;
  - a seeded random-input test (seed logged), with no panic;
  - a writer test for declared field order and escaping.
- [x] `rail-diag` tests:
  - diagnostic JSON matches the C3 shape;
  - sort order (module, def, path, span, rule);
  - the registry holds exactly TY001, FX001 and SKL001;
  - registering SKL001 twice fails.
- [x] Run the unit-scoped command and record the failing output.
- Traces: NFR4.3, NFR4.4, NFR3.1, BR4.1, BR4.3, BR4.4.

### Step 5 — Data model, Green

- [x] Implement `rail-json`:
  - a strict RFC 8259 reader using an explicit work stack, with a depth counter (64);
  - checked indexing and no panics;
  - a writer that keeps declared field order.
- [x] Implement `rail-diag`:
  - the `Diagnostic` type (rule, level `E`, loc {module, def, path, span}, message, confidence);
  - the fixed rule registry and the deterministic sort.
- [x] Unit-scoped command: the Step 4 tests pass.
- Traces: NFR4.3, NFR4.4, NFR3.1, BR4.1–BR4.4.

### Step 6 — Data model, Refactor

- [x] Tidy both crates with the tests green; `clippy -D warnings` clean.

### Step 7 — Repository / data access, Red: workspace access tests in `rail-tools`

- [x] Tests:
  - module path resolution inside the root;
  - rejection with `module.not_found` of `..`, absolute paths, backslashes, NUL, and a symlink pointing outside the root;
  - `module.unreadable` for an unreadable file;
  - `SKL001` at byte 16 MiB for an oversized file.
- [x] Run and record the failures.
- Traces: NFR5.3, NFR4.4, WF1 step 1.

### Step 8 — Repository / data access, Green

- [x] Implement path confinement (canonicalised root, symlink resolution, prefix check) and the bounded read (16 MiB + 1). Tests pass.

### Step 9 — Repository / data access, Refactor

- [x] Tidy with the tests green.

### Step 10 — Business logic, Red: front end, back end, runtime and operations

- [x] `rail-syntax` tests:
  - accepts both fixtures, and printing each reproduces it byte for byte (BR1.3);
  - SKL001 for an unsupported form (BR1.1), each layout violation (BR1.2), a malformed input with a precise location (BR1.4), a bad or duplicate ID (BR1.5), a reserved `skel` module (and `skeleton.answer` accepted) (BR1.6), nesting 257 (NFR4.4) and an integer suffix (BR2.4).
- [x] `rail-check` tests:
  - TY001 for a mismatch, differing arm types, a non-`bool` scrutinee, a bad operator argument, `main`'s shape, and misplaced `Caps`/`Result` (BR2.3, BR2.5–BR2.7);
  - FX001 for a missing `log` (BR3.2);
  - SKL001 for an unsupported type, an effect other than `log`, shadowing, an unused binding, an unknown name, a missing signature and a missing export (BR1.7, BR2.1, BR2.2, BR3.3);
  - `skel.print_i64` typing (BR3.1).
- [x] `rail-lower` and `rail-codegen` tests:
  - IR for literals, calls, `let` and the Boolean match;
  - overflow and division checks emitted (BR5.4);
  - an object file produced, with the same bytes twice (NFR3.2).
- [x] `rail-build` tests:
  - build and run `skeleton.answer`: prints `42`, exit 0;
  - an Err main: exit 1;
  - overflow and division by zero: exit 70 with one trap line (BR5.3, BR5.4, NFR4.2);
  - a blocked build returns `build.blocked`, and a module without `main` returns `build.no_entry` (BR4.2, BR2.8);
  - `cc` missing gives `build.failed` (NFR9.10);
  - a forced link failure leaves no artifact (NFR9.8);
  - two builds in two directories are byte-identical, and no absolute path appears in the binary (BR5.5, NFR3.2);
  - output lands under `.rail/build/dev/`.
- [x] `rail-tools` tests:
  - the four operations return the entity shapes;
  - the panic boundary gives `internal.error` (NFR9.7);
  - `run.failed` for the time limit, the output cap and a signal (NFR5.5, NFR9.9);
  - a cleared environment for the program (NFR5.1).
- [x] Run and record the failures.

### Step 11 — Business logic, Green: front end

- [x] Implement `rail-syntax`:
  - tokenizer and parser for the skeleton form set, with explicit nesting counters (256), layout validation and located `SKL001` diagnostics;
  - a printer for byte-exact round trips;
  - IDs and anchor paths.
- [x] Implement `rail-check`:
  - name resolution (locals, module functions, operators, `Ok`/`Err`, `skel.print_i64`);
  - monomorphic typing;
  - the `log` effect check;
  - the `main` shape check.
- [x] Front-end tests pass.

### Step 12 — Business logic, Green: back end and runtime

- [x] `crates/rail-runtime`: a `#![no_std]` Rust static library built with `panic = "abort"`. It declares `write` and `exit` itself (no `libc` crate), and every `unsafe` block carries a `// SAFETY:` comment. It provides:
  - the C `main` entry, which calls the generated `rail_entry` and exits with 0 or 1; on Err it writes `error: main returned Err` to stderr (E7);
  - `rail_rt_print_i64` (decimal, one `\n`, written out before exit) (BR5.7);
  - `rail_rt_trap(kind)`, which writes one trap line to stderr and exits with 70.
- [x] `rail-build`'s build script compiles `rail-runtime` with the same `rustc` (the `RUSTC` environment variable) for the host target into `OUT_DIR`, and embeds the archive bytes in `rail`. At link time, `rail` writes the archive into `.rail/build/dev/runtime/`, named by its content hash.
- [x] `rail-lower`: typed definitions become SSA IR, with checked arithmetic that branches to the trap entry.
- [x] `rail-codegen`: Cranelift for the host target, writing an object file through `cranelift-object`. No debug information.
- [x] `rail-build` (dev mode):
  1. check first;
  2. `.o.partial` then rename;
  3. `cc` started with an argument list, working directory set to the module's build directory, relative arguments;
  4. `.partial` executable then atomic rename;
  5. cleanup on failure;
  6. `cc` not found or failing mapped to `build.failed` (NFR5.4, NFR9.8, NFR9.10).
- [x] The run operation starts the program with no arguments, empty stdin and a cleared environment. It reads both pipes on threads, with a 16 MiB cap and a 10-second limit, and reports a signal as `run.failed` (NFR5.1, NFR5.5, NFR5.6, NFR9.9).
- [x] Back-end, runtime and `rail-build` tests pass.

### Step 13 — Business logic, Green: operations

- [x] `rail-tools`:
  - the four operations (parse, check, build, run) over workspace access;
  - the typed ToolError codes (including `internal.error`);
  - a `catch_unwind` panic boundary with a quiet panic hook (BR6.1, NFR9.6, NFR9.7).
- [x] Operation tests pass.

### Step 14 — Business logic, Refactor

- [x] Tidy all business-logic crates with the tests green; `clippy -D warnings` clean.

### Step 15 — API / endpoint, Red: protocol and command line

- [x] `rail-rap` tests:
  - framing (good; missing or duplicate `Content-Length`; a 1 KiB header; a 16 MiB + 1 body discarded in chunks);
  - `-32700`, `-32600`, `-32601`, `-32602` (missing or mistyped `module`), `rap.unsupported_param` (`depth`, `_deadline_ms`, an unknown key), `rap.not_initialized`, and a repeated `initialize` giving `-32600`;
  - notifications get no response;
  - the session continues after every error;
  - responses come back in request order;
  - stdout carries frames only;
  - a 1,000-request soak with the RSS check (BR6.3–BR6.5, NFR3.4, NFR9.5).
- [x] `rail` command-line tests:
  - `parse`, `check`, `build` and `run`, with exit codes 0, 1, 2 and 3, and the program's own code for `run` (BR6.2);
  - `--json` equals the protocol result (BR6.1);
  - `RAIL_LOG` off by default, and records with no timestamp or path (NFR9.13, NFR3.5).
- [x] Run and record the failures.

### Step 16 — API / endpoint, Green

- [x] Implement `rail-rap`:
  - the single-threaded request loop;
  - the layered validation from `security-design.md` (header, size, JSON, request, method, session, params shape, params support);
  - the session state machine.
- [x] Implement the `rail` command line (`parse`, `check`, `build`, `run`, `rap`, `--json`) and `RAIL_LOG` logging to stderr.
- [x] Protocol and command-line tests pass, and the Step 2 end-to-end tests now pass.

### Step 17 — API / endpoint, Refactor

- [x] Tidy with every test green.

### Step 18 — Frontend behavior

- Not applicable: the skeleton has no user interface. Its user-facing surfaces (the command line and the protocol) are covered by Steps 15–17.

### Step 19 — Environment and build configuration (CI, supply chain, fuzzing, verification)

- [x] `.github/workflows/ci.yml` (pull requests):
  - jobs `test (macos)` on `macos-latest` and `test (linux)` on `ubuntu-latest`: build, nextest with `--locked`, `fmt --check`, `clippy -D warnings`;
  - Linux coverage with `--fail-under-lines 80`;
  - `cargo deny check`;
  - `gitleaks`;
  - the `forbid(unsafe_code)` check script;
  - the runtime symbol allow-list check (`nm -u`);
  - the linked-libraries allow-list check (`otool -L` / `ldd`);
  - the `libfuzzer-sys`-not-in-`rail` dependency-tree check;
  - job `timeout-minutes: 30`.
  Actions are pinned by commit SHA, with `permissions: contents: read` (NFR10.1, NFR6.2, NFR7.5, NFR7.6, NFR5.2, NFR4.1, NFR9.1, NFR9.3).
- [x] `.github/workflows/nightly.yml`: the `json_parse` fuzz target for 30 minutes on the pinned nightly, uploading failing inputs, `timeout-minutes: 60` (NFR4.5).
- [x] `.github/workflows/weekly-advisories.yml`: `cargo deny check advisories` on a weekly schedule.
- [x] `deny.toml`: crates.io only; an allow-list of the locked Cranelift family and its transitive crates; licences compatible with MIT OR Apache-2.0, each checked against published metadata; advisories denied.
- [x] `fuzz/` (not a workspace member): `fuzz/Cargo.toml` with `libfuzzer-sys` (approved, fuzz-only) and `fuzz/fuzz_targets/json_parse.rs` (parse, write, re-parse, equal). Pin the nightly in `fuzz/rust-toolchain.toml`.
- [x] `scripts/verify-skeleton.sh`: a candidate for the recorded verification command. It runs the WF6 steps against a freshly built `rail` and prints `PASS n …` / `FAIL n …`, with the command and captured output on failure. It exits non-zero on any failure (NFR9.14). The command itself is chosen with you at the skeleton checkpoint.
- [x] Record the approved dependencies with their reasons in `docs/dependencies.md`: the Cranelift family and `libfuzzer-sys`.

### Step 20 — Documentation and traceability

- [x] `README.md`: a "Building the toolchain" section covering the prerequisites (rustup, `cc`), the test command, the bless switch and the verification script.
- [x] Short crate-level docs (`//!`) for each crate naming its building block and owning unit.
- [x] Tick every step in this plan as it is done.
- [x] Write `code-summary.md`, `source-manifest.json` (every created or modified path) and `traceability.json` (every BRx.y and NFRx.y mapped to an existing implementation or test file) in this unit's code-generation record directory.

## Traceability overview

| Requirement group | Steps |
|---|---|
| FR27 (walking skeleton end to end) | 2, 16 |
| BR1.1–BR1.7 (reading canonical text) | 10, 11 |
| BR2.1–BR2.8 (types, entry point) | 10, 11, 12 |
| BR3.1–BR3.3 (effects, temporary print) | 10, 11, 12 |
| BR4.1–BR4.4 (diagnostics) | 4, 5 |
| BR5.1–BR5.7 (build and run) | 10, 12 |
| BR6.1–BR6.6 (tool surface) | 13, 15, 16 |
| NFR1.x, NFR3.x, NFR4.x, NFR5.x, NFR6.x, NFR7.x, NFR9.x, NFR10.x | 1, 3, 4–5, 7–8, 10–13, 15–16, 19 |

## Assumptions & Open Questions

- [assumption] Cranelift can generate code for the host targets (AArch64 macOS, x86-64 Linux) with the four crates listed. If another Cranelift crate turns out to be needed (for example `cranelift-native` for host detection), it is part of the pre-approved Cranelift family and is added with its reason recorded in `docs/dependencies.md`.
- [assumption] A `no_std` Rust static library built by a build script with the workspace's own `rustc` links with `cc` on both platforms. If it does not, generation stops and the gap is brought to you. It is not worked around with a new dependency.
- Open: the exact recorded verification command is chosen at the skeleton checkpoint. `scripts/verify-skeleton.sh` is the proposed candidate.
