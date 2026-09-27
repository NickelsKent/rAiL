# Tech Stack Decisions — walking-skeleton (U1)

The skeleton creates the workspace every later unit fills in, so these decisions set the pattern: which tools and crates exist, where `unsafe` is allowed, and how quality is checked. Everything here follows the affirmed practices. The only new decisions are the ones confirmed in `nfr-requirements-questions.md`.

## Sources

- `requirements.md`: C1 (Rust, one Cargo workspace, one `rail` binary, pinned toolchains), NFR7 (dependencies), NFR9 (code quality), NFR10 (supply-chain hygiene).
- Team practices: Code Style, Testing Posture, Way of Working (CI checks), firm rules on dependencies and unsafe code.
- `components.md` (building blocks), `unit-of-work.md` (U1 owns thin versions and the workspace layout).
- `nfr-requirements-questions.md` [Q1]–[Q7], confirmed.

## Decisions

| Area | Decision | Why | Alternatives rejected |
|---|---|---|---|
| Language and edition | Rust, 2024 edition | Practice (C1) | — |
| Stable toolchain | Newest stable Rust on the day the workspace is created, pinned in `rust-toolchain.toml` with the `rustfmt`, `clippy` and `llvm-tools` components; bumps are deliberate pull requests [Q3] | Practice: one pinned stable for build, test and release | A version chosen now (Q3 B), since Rust is not installed yet and the newest stable avoids an immediate bump |
| Nightly toolchain | Newest nightly on the day the workspace is created, pinned separately and used only for the fuzz target [Q7] | Practice allows nightly for fuzzing only; Q6 brings fuzzing forward | Unpinned nightly (non-reproducible CI) |
| Workspace | One Cargo workspace, several crates, one `rail` binary (layout below) | Practice (C1) | A single crate (blurs the building-block boundaries later units extend) |
| Code generation | Cranelift, writing an object file [Q2] | Pre-approved; dev backend per spec §4.2 | — |
| Linking | Start the platform C compiler driver `cc` with an argument list, never a shell: Apple clang from the Xcode Command Line Tools on macOS, the system `cc` on Linux. `cc` is a documented build prerequisite for building rAiL programs [Q2] | `cc` knows each platform's start-up files, C library and linker flags | Calling `ld` directly (Q2 B): per-platform argument lists that break across OS versions |
| Runtime | A `#![no_std]` Rust static library that declares the few C functions it needs (`write`, `exit`) itself, with no `libc` crate. It is built with `panic = "abort"`, and `unsafe` is allowed only there, with safety comments | NFR7.1: nothing beyond the platform C library | The Rust standard library in the runtime (a larger binary and many more C-library symbols); the `libc` crate (a dependency for a handful of declarations) |
| JSON | In-house, strict RFC 8259 reader and writer in its own crate, with fixed limits and a fixed output key order [Q1] [Q6] | Your choice: no JSON crate | `serde` + `serde_json` (Q1 A) |
| JSON assurance | JSONTestSuite cases kept in the repository as test data; a seeded random-input test on stable in every CI run; a coverage-guided fuzz target in a separate fuzz crate using `libfuzzer-sys`, run briefly in nightly CI [Q6] [Q7] | Untrusted input at the protocol boundary | Unit tests only (Q6 C) |
| Golden files | In-house test helper: compares output with a committed expected file and rewrites it only when `RAIL_BLESS=1` is set, so every rewrite shows up as a reviewed diff [Q4] | Practice: explicit bless step | `insta` (Q4 B) |
| Formatting and lints | `rustfmt` (defaults); `clippy` on all code including tests, with warnings as errors; `#![forbid(unsafe_code)]` in every crate except `rail-runtime` and `rail-codegen` | Practice (Code Style) | — |
| Coverage | `cargo-llvm-cov` (a tool installed in CI, not a workspace dependency), at least 80% line coverage on Linux, excluding the fuzz crate, test helpers and golden files | Practice (Testing Posture) | `tarpaulin` (Linux-only, less accurate) |
| CI | GitHub Actions on GitHub-hosted macOS (Apple silicon) and Linux (x86-64) runners. Every pull request runs build, tests (`--locked`), clippy, rustfmt check, coverage, `cargo-deny` and `gitleaks`. The nightly run adds the fuzz target. `cargo-deny` advisories also run weekly. Actions are pinned to exact commits with read-only permissions | Practices (Way of Working, Testing Posture) | Self-hosted runners (the owner's machines are for the release check only) |

## Workspace layout

Crate names follow the building blocks in `components.md`. Each crate is a thin first version its owning unit extends. Exact module structure inside each crate is left to Code Generation.

| Crate | Building block | Kind | `unsafe` |
|---|---|---|---|
| `rail` | Cli, entry point for `rail rap` | binary | forbidden |
| `rail-syntax` | Syntax | library | forbidden |
| `rail-check` | QueryEngine + TypeChecker (skeleton subset) | library | forbidden |
| `rail-diag` | LintEngine: diagnostic shape and rule registry | library | forbidden |
| `rail-lower` | Lowering | library | forbidden |
| `rail-codegen` | CraneliftBackend | library | allowed, with safety comments, only if Cranelift's API requires it |
| `rail-build` | BuildDriver (dev mode, linking through `cc`) | library | forbidden |
| `rail-tools` | ToolServices | library | forbidden |
| `rail-rap` | RapServer | library | forbidden |
| `rail-json` | JSON reader and writer | library | forbidden |
| `rail-runtime` | RuntimeCore (skeleton subset) | `no_std` static library | allowed, with safety comments |
| `fuzz/` (outside the shipped binary) | Fuzz target for `rail-json` | fuzz crate | — |

## Dependencies

Firm rule: every new crate has a stated reason and your approval. Cranelift is pre-approved but still gets a recorded entry with its reason.

| ID | Crate or tool | Where | Reason | Approval |
|---|---|---|---|---|
| NFR7.4 | `cranelift-codegen`, `cranelift-frontend`, `cranelift-module`, `cranelift-object` (the Cranelift family, one pinned version) | `rail-codegen` | Dev-mode native code generation and object-file output (FR23, FR27); the smallest set of Cranelift crates that builds functions and writes an object file | Pre-approved (requirements Q5) |
| NFR7.5 | `libfuzzer-sys` | `fuzz/` only | Coverage-guided fuzzing of the in-house JSON parser at the protocol boundary | Approved [Q7] |
| NFR7.6 | Runtime dependencies: none. The linked skeleton program shows only operating-system libraries in `otool -L` (macOS) and `ldd` (Linux) | `rail-runtime`, built programs | NFR7.1, NFR7.3 | Practice |
| — | Tools: `cargo-llvm-cov`, `cargo-deny`, `cargo-fuzz`, `gitleaks`, `cc` | CI and developer machines | Coverage, supply-chain checks, fuzzing, secret scanning, linking | Tools, not workspace dependencies |

Transitive crates pulled in by the Cranelift family appear in `Cargo.lock`, and `cargo-deny` checks them for advisories, sources and licences. Adding any crate beyond the table above needs its own reason and your approval.

## Quality requirements carried by these decisions

| ID | Requirement | Verification |
|---|---|---|
| NFR9.1 | At least 80% line coverage of the Rust code on Linux before merge, excluding the fuzz crate, test helpers and golden files. The floor can go up but never down. | `cargo-llvm-cov` gate in CI |
| NFR9.2 | Tests come first. The skeleton's end-to-end check (WF6) is written first and fails; then each crate's tests are written and fail before its code. | Pull-request history and review |
| NFR9.3 | `rustfmt --check` and `clippy -D warnings` (including tests) pass on both platforms. | CI |
| NFR9.4 | Expected-output files change only when `RAIL_BLESS=1` is set, and the resulting diff is part of the pull request. | Test helper behaviour; review |
| NFR6.2 | Every pull request's CI runs on both macOS and Linux, and merging needs both to be green. | Branch protection and CI configuration |

## Assumptions & Open Questions

- [assumption] GitHub-hosted macOS runners are Apple silicon (AArch64) and Linux runners are x86-64, matching the two required platforms. This is checked when CI is set up.
- [assumption] The licences of the Cranelift crates, `libfuzzer-sys` and the JSONTestSuite files are compatible with MIT OR Apache-2.0. Each is checked against its published metadata when it is added, and `cargo-deny`'s licence list is extended only for a licence that fits.
- Rust is not installed on this Mac. Installing `rustup` is a prerequisite for Code Generation. The first pinned versions are chosen on that day.
