# NFR Requirements — walking-skeleton (U1) — Questions

Most of the skeleton's quality requirements are already settled. They come from the requirements (determinism, portability, a runtime with no dependencies, 80% coverage, CI on macOS and Linux), from the firm rules, and from the functional design. The skeleton is not a performance milestone: the speed targets apply to the release toolchain, and only numbers from your reference machines count.

What is still open is mainly the first crates and tools the workspace brings in. Every new crate needs a stated reason and your approval. Note: Rust is not installed on this Mac yet (no `rustc`, `cargo` or `rustup`). Code Generation will need it.

## Q1. How should the toolchain read and write JSON for the agent protocol and `--json` output?

The protocol server parses JSON from agents, which is untrusted input at a trust boundary. The command line's `--json` must print exactly the protocol result.

A. Use the `serde` and `serde_json` crates (compiler-side only; the runtime stays dependency-free). Reason: a JSON parser at a trust boundary is exactly the kind of code that is risky to write ourselves, and these crates are widely used and heavily fuzzed (recommended)
B. Write a small JSON reader and writer inside the toolchain, adding no crate
X. Other (please specify)

[Answer]: B

## Q2. How should the skeleton link native programs?

Cranelift produces machine code. Something still has to link it with the small runtime and the platform C library into an executable.

A. Have Cranelift write an object file (the `cranelift-object` part of the pre-approved Cranelift), then call the platform's C compiler driver `cc` to link. That is Apple's clang from the Xcode Command Line Tools on macOS, and the system `cc` on Linux. `cc` becomes a documented build prerequisite (recommended)
B. Call the system linker `ld` directly, with arguments built by hand for each platform
X. Other (please specify)

[Answer]: A

## Q3. Which stable Rust version should the workspace pin?

Practice: one pinned stable version for building, testing and releases. The separate pinned nightly for fuzzing and sanitizers is not needed until later units.

A. The newest stable Rust release on the day the workspace is created, recorded in `rust-toolchain.toml`; later bumps are deliberate changes (recommended)
B. A specific version you name
X. Other (please specify)

[Answer]: A

## Q4. How should expected-output ("golden file") tests be kept?

Practice: expected-output files change only through an explicit "bless" step, and the diff is reviewed in the pull request. The skeleton needs golden files for diagnostics, protocol responses and command output.

A. A small in-house test helper compares output with the expected file and rewrites it only when a bless switch (for example `RAIL_BLESS=1`) is set; no test crate (recommended)
B. The `insta` snapshot-testing crate (test-only; still needs a stated reason and your approval)
X. Other (please specify)

[Answer]: A

## Q5. Should the skeleton have any performance requirement?

A. No performance targets. The only timing requirement is a hang guard: the end-to-end check and every test must finish within a fixed time limit in CI, so a stuck process fails instead of hanging (recommended)
B. As A, and CI also records the skeleton's build time and program start-up time on every run as an informal baseline. It is never a gate, and never evidence for the release targets
X. Other (please specify)

[Answer]: A

## Follow-up questions

Your answer to Q1 (B) means the protocol server parses untrusted JSON with our own code. The design needs a clear assurance bar for that parser.

## Q6. How much assurance should the in-house JSON parser have in the skeleton?

A. A strict parser for the JSON standard (RFC 8259) with fixed limits: a maximum message size and a maximum nesting depth, with anything over them rejected. Tested with the public JSONTestSuite cases, stored in the repository as test data (a data file, not a crate), plus a seeded random-input robustness test that runs on stable Rust in every CI run. Coverage-guided fuzzing is added when the nightly fuzz lane arrives (recommended)
B. As A, plus a coverage-guided fuzz target now. This needs the pinned nightly and the `libfuzzer-sys` crate earlier than planned, and each needs your approval
C. Ordinary unit tests only for now; the parser is hardened when the agent loop (U5) completes the protocol server
X. Other (please specify)

[Answer]: B

## Q7. Do you approve the two additions that Q6 (B) brings forward?

Firm rule: every new Rust dependency needs a stated reason and your approval before it is added.

- **`libfuzzer-sys`.** Fuzz-only. It lives in a separate fuzz crate that is never part of the shipped `rail` binary and is excluded from coverage. Reason: coverage-guided fuzzing of the in-house JSON parser, which reads untrusted agent input at the protocol boundary.
- **A pinned nightly Rust.** Used only for the fuzz target, as the practice already allows for fuzzing. It is pinned to the newest nightly on the day the workspace is created. The `cargo-fuzz` tool is installed in CI and on developer machines; it is not a workspace dependency.

A. Approve both; a short fuzz run of the JSON parser is added to the nightly CI run (recommended)
B. Not now; use Q6 (A) instead, and add the fuzz target when the nightly fuzz lane arrives
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
