# Code Summary — walking-skeleton (U1)

The walking skeleton is built on branch `bolt/walking-skeleton`. One Cargo workspace produces one `rail` binary. It parses, checks, serves over RAP, builds with Cranelift, links with `cc` and a `no_std` runtime, and runs `skeleton.answer`, which prints `42` and exits 0. `skeleton.broken` is refused with its one `TY001` through the command line and through the protocol. All 20 plan steps are done and ticked. Everything below was verified on macOS (AArch64). Linux (x86-64) is written for but **not verified**; see "Unverified items".

## Files created and modified

| Area | Paths |
|---|---|
| Workspace | `Cargo.toml` (11 members; `rail-runtime` and `fuzz/` excluded), `Cargo.lock`, `rust-toolchain.toml` (1.98.1 with rustfmt, clippy, llvm-tools-preview), `clippy.toml` (HashMap/HashSet disallowed), `.config/nextest.toml` (30 s × 2), `LICENSE-MIT`, `LICENSE-APACHE`, `.gitignore` (adds `/target/`, `.rail/`) |
| Data model | `crates/rail-json` (strict RFC 8259 reader with an explicit work stack and depth 64, ordered writer), `crates/rail-diag` (C3 `Diagnostic`, `CheckResult`, sort, registry with TY001/FX001/SKL001) |
| Front end | `crates/rail-syntax` (`reader.rs` layout + S-expressions with a 256 nesting counter, `tree.rs` forms/IDs/literals, `print.rs` byte-exact printer), `crates/rail-check` (`sig.rs` signatures, `body.rs` bidirectional typing, scopes, `log` effect) |
| Back end | `crates/rail-lower` (SSA IR with checked arithmetic and `rail_entry`), `crates/rail-codegen` (Cranelift 0.136.1 to a host object), `crates/rail-build` (build script compiles the runtime; atomic `.partial` writes; `cc` with an argument list and relative paths), `crates/rail-runtime` (`no_std` static library: C `main`, `rail_rt_print_i64`, `rail_rt_trap`) |
| Operations | `crates/rail-tools` (workspace confinement and bounded reads, `ToolError` with `internal.error`, the four operations behind `catch_unwind`, bounded child-process runner, tree JSON, log records) |
| Surfaces | `crates/rail-rap` (framing, 8 validation layers, session state machine), `crates/rail` (CLI: `parse`/`check`/`build`/`run`/`rap`, `--json`, `RAIL_LOG`) |
| Test tooling | `crates/rail-testkit` (golden files with `RAIL_BLESS=1`, 60 s process runner, framed RAP client, helper binaries `never-ends`, `floods-output`, `aborts`) |
| Test data | `fixtures/skeleton/answer.rlc`, `fixtures/skeleton/broken.rlc`, `crates/rail/tests/golden/` (5 hand-written goldens and `tree-answer.json`), `tests/data/JSONTestSuite/` (318 cases, upstream commit `1ef36fa01286573e846ac449e8683f8833c5b26a`, MIT) |
| CI and supply chain | `.github/workflows/ci.yml`, `nightly.yml`, `weekly-advisories.yml`; `deny.toml`; `fuzz/` (`json_parse` target, `libfuzzer-sys` =0.4.13, nightly-2026-09-27); `scripts/check-forbid-unsafe.sh`, `check-runtime-symbols.sh`, `check-linked-libs.sh`, `allowlists/`, `verify-skeleton.sh` |
| Docs | `README.md` ("Building the toolchain" section), `docs/dependencies.md`, crate-level `//!` docs in every crate |

## Key decisions

- **Runtime linking.** The runtime is compiled by `rail-build`'s build script with the workspace `rustc`, `-C panic=abort -C lto=fat`, into `OUT_DIR`, and embedded in `rail`. Without fat LTO, `libcore` objects reference `rust_eh_personality` and the link fails. With it, a linked program imports only `_exit` and `_write` and links only `/usr/lib/libSystem.B.dylib` (checked with `nm -u` and `otool -L`). `--remap-path-prefix` keeps local source paths out of the archive. The archive is written to `.rail/build/dev/runtime/rt-<FNV-1a-64>.a`.
- **Cranelift without `cranelift-native`.** The ISA is looked up by its fixed triple (`aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`) with baseline features, `opt_level=none`, `is_pic=true`, `unwind_info=false`. This needs no fifth crate and keeps object bytes identical across machines. Overflow checks use plain instructions (xor/and sign test, `smulhi` against the sign of the low half, and MIN/-1 for division). Runtime imports are declared only when used.
- **Determinism.** `cc` runs in the module's build directory with relative arguments and `-Wl,-S` (no debug info or stabs). Two builds in two directories are byte-identical, and no absolute path appears in the binary (tested).
- **Value representation.** Every skeleton value is an `i64`: bool is 0/1, unit and Caps are 0, and main's Result is its tag. `rail_entry` calls main with Caps 0 and returns the tag; the runtime's C `main` maps it to exit 0 or 1.
- **Anchor paths.** Paths are dot-separated child indexes. A form's children are the elements after its keyword; every element of an application is a child. In a `fn` item, `0` is the ID, `1` the name, `2` the parameters, `3` the signature and `4` the body. The broken fixture's TY001 is at `#sk0a1b/4` [82,90].
- **Module arguments** are workspace-relative paths with `.rlc` (`skeleton/answer.rlc`). The qname is derived from the path and must match the `mod` item. Path segments must be lowercase names, which makes `..`, absolute paths, backslashes and NUL invalid by construction. Symlinks are confined with a canonical prefix check.
- **Result shapes.** `tree.get`: `{module, tree, diagnostics}`. `check.run`: `{module, diagnostics, blocking}`. `build.run`: `{module, target, mode, path}`. `run.run`: `{exit_code, stdout, stderr}`. A tool error over RAP is `-32000` with the ToolError `{code, message, data?}` as `data`; `rail --json` prints `{"error": <ToolError>}`.
- **Logging.** `RAIL_LOG=error|info|debug` writes `rail <level> <operation> <module> [code=<code>] <message>` to stderr. A bad value gives one warning line. Records carry no time, PID, host or absolute path. CLI and RAP share `perform_logged`.
- **Stricter than required.** Non-ASCII source is rejected as SKL001, which makes NFC trivially true. JSON-RPC ids must be a string or an integer.

## Tests, coverage and results

- **121 tests, all passing** (`cargo nextest run --locked` with the unit-scoped package list): `rail` 17 (8 WF6 e2e, 8 CLI, 1 soak of 1,000 requests with an RSS check), `rail-json` 12 (the JSONTestSuite runner covers all 318 cases), `rail-diag` 7, `rail-syntax` 11, `rail-check` 13, `rail-lower` 8, `rail-codegen` 5, `rail-build` 10, `rail-tools` 17, `rail-rap` 11, `rail-testkit` 10 (4 of them run-limit tests).
- **Line coverage 91.98%** (3,517 lines, 282 missed; `cargo llvm-cov nextest` with the approved coverage command, **measured on macOS**; the gate is on Linux in CI). The lowest file is `rail-tools/src/run.rs` at 59%, because its limit tests live in `rail-testkit`, which the approved coverage command excludes.
- `cargo fmt --check`, `cargo clippy --all-targets -D warnings` (workspace and `rail-runtime`), `cargo deny check` (advisories, bans, licences, sources ok; one warning for duplicate `hashbrown` from upstream), `scripts/check-forbid-unsafe.sh`, `check-runtime-symbols.sh` and `check-linked-libs.sh`: all pass on macOS.
- `scripts/verify-skeleton.sh`: PASS 1–7 on macOS in about 2 s. A forced golden mismatch prints FAIL lines with the command and output, and the script exits 1.
- Fuzz smoke run: `cargo fuzz run json_parse` for 60 s gave 15.4 million executions and no finding.

## Red-step evidence (TDD)

Each Red step ran the unit-scoped command before its Green step. These notes belong under the plan steps, but adding any text other than a tick changes the plan's approval fingerprint, and the plan-approval guard then blocks work. So they are recorded here.

- **Step 3:** `14 tests run: 6 passed, 8 failed`. The 6 testkit tests pass; all 8 `rail::e2e` WF6 tests fail against the stub binary (for example `left: Some(3) right: Some(0)`).
- **Step 4:** `unresolved imports rail_json::{MAX_DEPTH, Number, Value, parse, to_string}` and `rail_diag::{CheckResult, Diagnostic, ...}`. The test targets `json_test_suite`, `json`, `random_input` and `diag` do not compile.
- **Step 7:** `unresolved imports rail_tools::{ErrorCode, MAX_SOURCE_BYTES, Workspace}`. The 7 workspace-access tests do not compile.
- **Step 10:** `rail-syntax`, `rail-check`, `rail-lower`, `rail-codegen`, `rail-build` tests and the `rail-tools` lib test do not compile (for example `unresolved imports rail_syntax::{ExprKind, MAX_NESTING, Module, Pattern, parse, print}`, `cannot find function guarded`).
- **Step 15:** `unresolved import rail_rap::serve` (11 protocol tests). `-p rail`: `17 tests run: 0 passed, 17 failed` (8 CLI, the soak and 8 e2e tests).

## Deviations from the plan

1. **Red notes** are recorded here, not under the plan steps (see above). The plan file differs from its approved text only by ticks.
2. **Nightly pin** is `nightly-2026-09-27`, not `nightly-2026-09-26`. The installed nightly's channel manifest is dated 2026-09-27, and that name resolves to exactly `rustc 1.101.0-nightly (75a75c3e0 2026-09-26)`, the version the brief recorded.
3. **JSONTestSuite:** `y_object_duplicated_key.json` and `y_object_duplicated_key_and_value.json` are asserted to be *rejected* with the duplicate-key error, because NFR4.3 requires it. This is documented in `SOURCE.md` and the test.
4. **The e2e tests compare exact response bytes** instead of parsing JSON, so they could compile at Step 3 before `rail-json` existed. This is also stricter, because it pins field order.
5. **Where tests live:** the run-limit tests (time limit, output cap, signal, cannot start) are in `crates/rail-testkit/tests/run_limits.rs`, because only a package's own tests can locate its helper binaries (`CARGO_BIN_EXE_*`). `rail-testkit` therefore dev-depends on `rail-tools`. The RSS soak test is in `crates/rail/tests/rap_soak.rs` because it needs the `rail` binary. The `internal.error` fault injection is tested at the `rail-tools` boundary; the protocol maps every ToolError to `-32000` the same way.
6. **Runtime symbol check** runs `nm -u` on a linked sample program rather than on the archive. The LTO'd archive contains `compiler_builtins` members whose references are platform noise, while the program shows exactly what the runtime takes from the C library.
7. **`.gitignore`** ignores `.rail/` at any depth (the plan said `/.rail/`), so `fixtures/.rail/` from the README example is also ignored.
8. **Small technical changes:** the `[profile.test] panic` setting was dropped because Cargo ignores it (tests always unwind). `rail_syntax::parse` returns `Box<Diagnostic>` to satisfy `clippy::result_large_err`. `rail-runtime` has its own `Cargo.toml` and `Cargo.lock`, used only for linting (it is not a workspace member).
9. **`tree-answer.json`** is the one golden not written by hand. It was created with `RAIL_BLESS=1`, reviewed, and every one of its 22 node spans was checked mechanically against the fixture bytes. The other five goldens were written before the code and matched on the first Green run.

## Unverified items and follow-ups

- **Linux has not been run.** This covers the Linux CI job, the 80% coverage gate as measured on Linux, x86-64 code generation and trap behaviour, the glibc start-up symbols in `scripts/allowlists/runtime-symbols-linux.txt`, and the `ldd` allow-list. No Linux machine or container was available.
- **GitHub Actions has not been run.** This includes `rustup toolchain install` (no arguments, reading `rust-toolchain.toml`), `taiki-e/install-action` with pinned tool versions, and `gitleaks-action` v3 with comments off. Every action is pinned to a commit SHA read from the GitHub API: checkout v7.0.1 `3d3c42e5…`, upload-artifact v7.0.1 `043fb46d…`, install-action v2.87.21 `4cef1412…`, gitleaks-action v3.0.0 `e0c47f4f…`.
- **Branch protection** (both `test (macos)` and `test (linux)` required) is a repository setting for the owner to apply.
- **README:** the existing line "Status: specification only. No compiler exists yet." was left unchanged. It is now out of date, and the owner may want to reword it.
- **Verification command:** the recorded command is still to be chosen at the skeleton checkpoint. `scripts/verify-skeleton.sh` is the candidate.
