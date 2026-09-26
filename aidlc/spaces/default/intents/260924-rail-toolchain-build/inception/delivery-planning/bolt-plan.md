# Bolt Plan — rAiL Toolchain

A **Bolt** is one build pass over one unit of work that ends in something that runs and is tested. This plan orders the 17 units from `unit-of-work.md` into 17 Bolts, one unit per Bolt [Q2]. Construction runs them one at a time, in this order. Each Bolt ends with its own verification and your sign-off.

The **walking skeleton** comes first. It is a thin slice that runs end to end and proves the pieces connect before real features go in. The order after it follows the spec's phase order [Q1], with three agreed moves [Q4] and the two "may slip" units last [Q3].

Why this order: see `risk-and-sequencing-rationale.md`. Owners: see `team-allocation.md`. Outside dependencies: see `external-dependency-map.md`. Sources: `unit-of-work.md`, `unit-of-work-dependency.md`, `unit-of-work-story-map.md` (FR IDs), `contract-summary.md` (contract IDs), `team-practices.md`, and `delivery-planning-questions.md` ([Q1]–[Q5]).

## Construction settings used by this plan

| Setting | Value | Source |
|---|---|---|
| Iteration | Unit by unit: each unit goes through its design stages, tests and code, then build and test, before the next starts | Workflow state (unit-major) |
| Execution | Serial, one Bolt at a time | Workflow state; one builder |
| Checkpoints | Enabled: every completed unit is verified with the recorded verification command and signed off | Workflow state |
| Walking skeleton | On: Bolt 1 is the skeleton, and it needs your checkpoint approval before Bolt 2 starts | Walking Skeleton practice |
| Branching | One short-lived branch per Bolt from `main`; pull request with green CI, AI review and your approval; squash-merged as one commit named after the Bolt | Way of Working practice |
| Testing | Tests first. Each Bolt's acceptance suites go into the harness and fail before its code is written | Testing Posture practice |

## Bolt sequence

### Bolt 1 — walking-skeleton (U1) · walking skeleton

- **Delivers:** a one-function canonical rAiL module that is parsed, formatted, checked, served through RAP, lowered, compiled with Cranelift, linked with a minimal runtime, and run as a native binary on macOS and Linux. [FR27]
- **Definition of done:**
  - The recorded verification command builds and runs the module on both platforms, and prints the expected output.
  - CI is green on macOS and Linux.
  - The one-workspace, several-crates layout is in place.
  - Coverage is at least 80% on Linux.
  - You have approved the skeleton checkpoint.
- **Confidence hypothesis:** the chosen pipeline really connects end to end. Canonical text → checker → RAP → Cranelift → runtime → native binary works on both platforms without structural rework.
- **Expected demo:** `rail check` and `rail run` on the skeleton module from the command line, plus the same `check` through a RAP session. Both platforms show identical output.

### Bolt 2 — acceptance-harness (U2)

- **Delivers:** the T1–T14 harness framework, with recorded CI volumes and seed policies (C9), driving `rail` as a black box. [FR57]
- **Definition of done:**
  - A sample suite runs at CI volume and at full volume.
  - Volumes and seeds are recorded in the repository.
  - CI runs the harness on both platforms.
- **Confidence hypothesis:** every later unit can add its suites first, and a failing suite really blocks the merge.
- **Expected demo:** a deliberately failing sample suite blocks a pull request in CI, then passes after a fix.

### Bolt 3 — benchmark-corpus (U16)

- **Delivers:** the 40 programs in rAiL, Rust, Go and OCaml, in the C10 layout, with fairness reviews, registered as harness inputs. [FR54]
- **Definition of done:**
  - All 40 programs exist in all four languages.
  - The Rust, Go and OCaml versions build and run with pinned toolchains.
  - The rAiL versions are written in canonical text from the spec.
  - Fairness reviews are recorded.
- **Confidence hypothesis:** the corpus covers the four program classes and gives later suites (T3, T4, T7, T9, T10, T13) real inputs.
- **Expected demo:** corpus listing plus the Rust twin of each program passing its expected outputs.

### Bolt 4 — syntax (U3)

- **Delivers:**
  - The shared grammar and the three codecs.
  - IDs and hashes, `view`/`absorb`, and source maps.
  - The grammar card and the constrained-decoding grammar.
  - The `parse`, `fmt`, `view` and `absorb` commands.
  - [FR1–FR6, FR17, FR18]
- **Definition of done:**
  - T1, T2 and T4 pass at CI volume on both platforms, including T4 over the corpus rAiL programs.
  - The generated-artifacts-up-to-date check passes.
  - Contracts C1 and E3 are implemented as pinned.
- **Confidence hypothesis:** generating every parser and printer from one grammar really makes round trips correct by construction.
- **Expected demo:** `rail view` then `rail absorb` of the three spec examples reproduces the canonical bytes. A mutated file is rejected with a located `FMT` error and an autofix.

### Bolt 5 — checker-and-lints (U4)

- **Delivers:**
  - The incremental query engine.
  - Type and effect checking.
  - The lint engine with the Phase 1 families and overrides.
  - [FR7–FR12]
- **Definition of done:**
  - Checker tests cover every Phase 1 rule.
  - The diagnostic stream matches the C3 schema.
  - Incremental re-check touches only edited definitions and their dependents.
  - Contracts C2 and C3 are implemented as pinned.
- **Confidence hypothesis:** HM with effect rows gives local, readable errors, and incremental checking is fast enough to approach the 50 ms target.
- **Expected demo:** `rail check` on the spec examples. Example 2 shows the `REC001` finding with its autofix, and effect rows are inferred correctly.

### Bolt 6 — agent-loop (U5) · first usable slice complete

- **Delivers:**
  - The edit engine.
  - The semantic diff, and the concise and reviewer explanations.
  - The service layer, with the CLI (`--json`) and RAP (`Content-Length` framing, `initialize`, cancellation, deadlines).
  - [FR13–FR16, FR19, FR20]
- **Definition of done:**
  - T5 and T6 pass at CI volume.
  - Contracts C4, E1, E2 and E4 are implemented as pinned.
  - A malformed request never ends the server.
  - Stdout carries only protocol messages.
- **Confidence hypothesis:** your own agents can write, check, patch, merge and review canonical rAiL end to end. This is the fallback product if the rest takes longer than hoped.
- **Expected demo:** an agent session over RAP that applies a patch, gets the diagnostic delta, runs a three-way merge of two disjoint patches, and produces a reviewer packet.

### Bolt 7 — execution-core (U6)

- **Delivers:**
  - Core, Mono and RIR lowering with Perceus reference counting.
  - The full Cranelift backend.
  - The build driver's dev mode.
  - The complete runtime core, with a from-scratch allocator.
  - [FR21–FR24]
- **Definition of done:**
  - Compiled programs run on both platforms.
  - Tail-call tests pass per platform.
  - The runtime has no dependencies beyond libc (CI check).
  - Contracts C5 and C6 are implemented as pinned.
- **Confidence hypothesis:** reference counting with borrow and reuse inference gives correct programs with no leaks, and the RIR and runtime ABI support a second backend.
- **Expected demo:** the spec's Example 1 and Example 2 (after autofix) compiled and run natively. Example 2 handles a one-million-line input in constant stack.

### Bolt 8 — release-backend (U12) · pulled forward

- **Delivers:**
  - The LLVM backend (O2, ThinLTO, `musttail`).
  - The full fixed-order optimization pipeline.
  - The build driver's release mode.
  - [FR48, FR49]
- **Definition of done:**
  - Release builds of the available programs are byte-identical across repeat builds.
  - Tail calls are guaranteed.
  - First speed and size measurements are recorded against the Rust twins, using the programs that run so far.
- **Confidence hypothesis:** the §11.2 speed and size targets are reachable with this design. If not, the gap shows up early enough to change the design.
- **Expected demo:** a release build of the available corpus programs, with a first speed and size comparison against Rust.

### Bolt 9 — system-and-stdlib (U7)

- **Delivers:**
  - The system interface, capabilities, the run manifest, and native runs with the no-sandbox warning.
  - The core standard library, the Untrusted and Secret types, and linear handles.
  - The test runner and the profiler.
  - [FR25, FR26, FR30, FR33–FR35]
- **Definition of done:**
  - T9, T10 (CI volume) and T12 pass on both platforms.
  - An empty manifest grants nothing.
  - Contract C7 and the manifest format are implemented as pinned.
- **Confidence hypothesis:** the capability model holds up against escape attempts (T12), and the standard library lets the corpus run.
- **Expected demo:** the spec's Example 3 run natively with a narrowed manifest. A write outside the grant is denied with a typed error, and `rail prof` shows reference-counting share per definition.

### Bolt 10 — measurement-tools (U17) · pulled forward

- **Delivers:**
  - `rail-perf`.
  - Tokenizer counts across at least three model families.
  - Generation-reliability runs (parse-valid and check-pass rates for rAiL vs Rust).
  - [FR55, FR56]
- **Definition of done:**
  - The tools run end to end on your reference machines.
  - A first measurement report is recorded.
  - API keys are kept out of the repository and saved output.
- **Confidence hypothesis:** the agent-reliability goal (at least 10 points above Rust's check-pass rate) is on track, or the gap is known early. The encoding gate itself is decided only after the full toolchain exists.
- **Expected demo:** a first report of token counts and check-pass rates for rAiL canonical text, rAiL human form, and Rust.

### Bolt 11 — concurrency-and-system-std (U8)

- **Delivers:**
  - The scheduler and I/O reactor (io_uring, kqueue).
  - Structured tasks and replay.
  - `std.task`, `std.par`, `std.proc`, `std.env`, `std.time`, `std.rand` and `std.log`.
  - [FR28, FR29, FR31]
- **Definition of done:**
  - T7 passes at CI volume on both platforms.
  - Replay reproduces recorded runs exactly.
- **Confidence hypothesis:** deterministic concurrency holds: output is identical across runs, core counts and platforms.
- **Expected demo:** the data-parallel corpus class run at different core counts with identical output, and a replayed run of a program using clocks and randomness.

### Bolt 12 — crypto-and-net (U9) · confirmed next after concurrency

- **Delivers:**
  - From-scratch cryptographic primitives.
  - A TLS 1.3 client and server.
  - `std.net` (TCP, UDP, DNS, TLS).
  - [FR32]
- **Definition of done:**
  - Published test vectors pass for every primitive.
  - Interoperability passes against two independent TLS implementations.
  - Constant-time checks pass.
  - TLS parsers are in the fuzz set.
  - Contract C8 is implemented as pinned.
- **Confidence hypothesis:** a from-scratch TLS stack can meet the agreed verification bar.
- **Expected demo:** a rAiL client and server completing TLS 1.3 handshakes with both independent implementations.

### Bolt 13 — supply-chain (U10)

- **Delivers:**
  - Packages, minimal version selection, the lockfile, cache and vendoring.
  - Semver publishing.
  - Signing, the transparency log, audit, and the local registry.
  - Reproducible builds and provenance, and `rail verify`.
  - FFI modules.
  - [FR38–FR46]
- **Definition of done:**
  - T3 and T14 pass on both platforms.
  - Contracts C11 and E5 are implemented as pinned.
  - End-to-end publish, resolve, download and verify works against a local registry.
- **Confidence hypothesis:** builds are byte-for-byte reproducible and fully offline, and the trust chain rejects tampered packages.
- **Expected demo:** publish a package to `rail registry serve`, then resolve and build a dependent offline. `rail verify` reproduces the artifact, and a tampered archive is rejected.

### Bolt 14 — first-party-packages (U11)

- **Delivers:** `rail/http` and `rail/zstd`, published to the local registry. [FR47]
- **Definition of done:** both packages pass their tests, are published and resolvable, and use no FFI.
- **Confidence hypothesis:** the package system and standard library are enough to build real libraries.
- **Expected demo:** a small rAiL program fetching a page over HTTPS with `rail/http` and compressing it with `rail/zstd`.

### Bolt 15 — deep-analysis (U13)

- **Delivers:**
  - The remaining lint families and their analyses.
  - The bounded model checker.
  - The developer and audit explanations and derived views.
  - `rail equiv`.
  - [FR50–FR52]
- **Definition of done:**
  - T8 passes (recall and false-positive thresholds).
  - The verifier finds known contract violations.
  - T13 passes at release volume on your reference machines. This is the release check for 1.0 performance.
- **Confidence hypothesis:** the deep analyses catch the seeded defects at the required recall without slowing the agent loop past its targets.
- **Expected demo:** `rail check` on the seeded-defect suite showing the findings, and `rail verify --bmc` on a contract with a counterexample.

### Bolt 16 — debugger (U14) · may slip

- **Delivers:** a DAP-compatible `rail debug` with breakpoints on anchor paths. [FR53]
- **Definition of done:** a DAP client can set an anchor breakpoint, stop there, and show values in human form.
- **Confidence hypothesis:** anchor-based debugging works through source maps.
- **Expected demo:** stepping through the spec's Example 3 in an editor.

### Bolt 17 — wasm-sandbox (U15) · may slip

- **Delivers:**
  - The Wasm component target.
  - `rail run --sandbox` with limits.
  - The hash-chained audit log.
  - [FR36, FR37]
- **Definition of done:**
  - T11 passes.
  - T12 passes inside the sandbox.
  - The sandbox cold-start target is met on your reference machines.
- **Confidence hypothesis:** untrusted agent code can be run safely by default.
- **Expected demo:** an adversarial program (fork bomb, allocation bomb) stopped at its limit, with a complete audit log.

## After the last Bolt

The full-volume release check runs all 14 suites on your reference machines. The §11.3 canonical-encoding gate is then decided from Bolt 10's measurements, re-run on the finished toolchain [scope Q8]. The spec is updated with every decision made during the work (firm rule C).

## Assumptions & Open Questions

- [assumption] Bolt 8 runs before the standard library exists, so its first performance numbers cover only the corpus programs that need no standard-library features beyond what Bolt 7 provides. The full T13 check stays in Bolt 15.
- [assumption] Bolt 10 runs before concurrency and TLS exist, so its first reliability runs use generation tasks that do not need those features. The measurements are re-run on the finished toolchain for the gate decision.
- Open question: which command verifies each completed unit. A greenfield repository has no runnable check yet, so this may be chosen at the first checkpoint (see the delivery planning completion).
