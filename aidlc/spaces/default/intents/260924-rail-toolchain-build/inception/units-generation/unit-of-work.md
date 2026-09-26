# Units of Work — rAiL Toolchain

The 32 building blocks from Domain Design (`components.md`) are grouped into 17 units of work. Each unit goes through Construction on its own. This document says what each unit is and owns. Dependencies are in `unit-of-work-dependency.md`, and the requirement-to-unit map is in `unit-of-work-story-map.md`. The build order is chosen in Delivery Planning. Sources: `components.md`, `decisions.md` (ADR-001 to ADR-011), `requirements.md` (FR/NFR IDs), `team-practices.md`, and `units-generation-questions.md` ([Q1]–[Q5]).

## Grouping approach

- **By capability.** About 17 units, grouped by capability. [Q1]
- **Components that grow across phases.** Some building blocks (Syntax, ToolServices, Cli, RapServer, BuildDriver, Translator, LintEngine) are built thin in one unit and extended by later ones. Each unit's table says which parts it owns.
- **Walking skeleton first, with the skeleton checkpoint on.** [Q2]
  - U1 is built completely through every Construction stage. It is verified with one command the owner approves, and the owner signs it off before any other unit starts.
  - The workflow reads this stance from the affirmed Walking Skeleton practice when Construction begins.
- **Acceptance harness.** It is its own unit. Every later unit adds its suites to it before writing code, because tests come first. [Q3]
- **Should items stay separate.** The debugger and the Wasm sandbox are separate units so they can slip. [Q5]
- **Deployment model.** Nothing is deployed to servers. The table below uses these values:
  - **embedded in `rail`**: part of the single binary.
  - **linked into programs**: statically linked into every compiled rAiL program.
  - **published package**: published to the local registry.
  - **project tooling**: in the repository, outside the binary.

## Units

| Unit ID | Directory | Unit | Kind | Deployment | Complexity |
|---|---|---|---|---|---|
| U1 | u1-walking-skeleton | walking-skeleton | service | embedded in `rail` + linked into programs | M |
| U2 | u2-acceptance-harness | acceptance-harness | library | project tooling | M |
| U3 | u3-syntax | syntax | library | embedded in `rail` | L |
| U4 | u4-checker-and-lints | checker-and-lints | library | embedded in `rail` | XL |
| U5 | u5-agent-loop | agent-loop | service | embedded in `rail` | L |
| U6 | u6-execution-core | execution-core | library | embedded in `rail` + linked into programs | XL |
| U7 | u7-system-and-stdlib | system-and-stdlib | library | linked into programs + embedded in `rail` | L |
| U8 | u8-concurrency-and-system-std | concurrency-and-system-std | library | linked into programs | L |
| U9 | u9-crypto-and-net | crypto-and-net | library | linked into programs (primitives also used by `rail`) | XL |
| U10 | u10-supply-chain | supply-chain | service | embedded in `rail` | XL |
| U11 | u11-first-party-packages | first-party-packages | library | published package | M |
| U12 | u12-release-backend | release-backend | library | embedded in `rail` | L |
| U13 | u13-deep-analysis | deep-analysis | library | embedded in `rail` | XL |
| U14 | u14-debugger | debugger (Should) | library | embedded in `rail` | M |
| U15 | u15-wasm-sandbox | wasm-sandbox (Should) | service | embedded in `rail` | L |
| U16 | u16-benchmark-corpus | benchmark-corpus | spec | project tooling | L |
| U17 | u17-measurement-tools | measurement-tools | library | project tooling | M |

## Unit definitions

### U1 — walking-skeleton (`u1-walking-skeleton`, service)

- **Description:** the smallest working integrated slice. A one-function canonical rAiL module is parsed, formatted and checked. It is served through the agent protocol server, lowered, compiled with Cranelift, linked with a minimal runtime, and run as a native binary on macOS and Linux. [FR27] [practices]
- **Owns (thin first versions):**
  - Syntax: canonical text parser and printer for the forms the example needs.
  - QueryEngine and TypeChecker: enough for one monomorphic function.
  - ToolServices, Cli and RapServer: `check`, `build`, `run` and `tree.get` only.
  - Lowering and CraneliftBackend: the forms the example needs.
  - BuildDriver: dev mode only.
  - RuntimeCore: allocation, traps, and exit from `main`.
- **Delivers:** the recorded end-to-end verification command, and the workspace layout (one Cargo workspace, several crates, one `rail` binary) that later units fill in.
- **Boundaries:** no feature completeness. Every part it touches is deepened by its owning later unit.
- **Notes:** this is the one agreed exception to phase order (it pulls a thin piece of Phase 2 forward). Tests come first here too.

### U2 — acceptance-harness (`u2-acceptance-harness`, library)

- **Description:** the T1–T14 acceptance harness framework. It runs any suite at its recorded CI volume or its full release volume. [FR57]
- **Owns:** AcceptanceHarness (SuiteConfig, SuiteRun), CI and release-check entry points, the recording of each suite's CI volume and seed policy, and black-box drivers for the CLI and RAP.
- **Boundaries:** it owns no suite content beyond the framework. Each later unit adds its own suites before its code, as the tests-first practice requires.
- **Notes:** CI volume reductions count as lowering a pass criterion (firm rule).

### U3 — syntax (`u3-syntax`, library)

- **Description:** full Phase 0 representation. [FR1–FR6, FR17, FR18]
- **Owns:**
  - Syntax, complete: shared grammar; the three generated codecs (`.rlc`, `.rlb`, `.rlh`); unique-spelling rules with autofix; IDs, anchors, `defhash` and `modhash`; `view` and `absorb`; source maps; the grammar card; the constrained-decoding grammar.
  - The Phase 0 CLI commands `parse`, `fmt`, `view` and `absorb`.
- **Suites added:** T1, T2, T4.

### U4 — checker-and-lints (`u4-checker-and-lints`, library)

- **Description:** Phase 1 checking. [FR7–FR12]
- **Owns:**
  - QueryEngine, complete.
  - TypeChecker, complete: types, effects, traits, exhaustiveness, effect-bound enforcement from workspace inputs (ADR-010), and public-API extraction.
  - LintEngine: rule registry, diagnostic schema, overrides, blocking policy, and the `FMT TY FX DEAD UNU` families.

### U5 — agent-loop (`u5-agent-loop`, service)

- **Description:** completes the first usable slice. Agents can write, check, lint, patch, merge and review canonical rAiL through RAP. [FR13–FR16, FR19, FR20]
- **Owns:**
  - EditEngine.
  - Translator, review part: semantic diff, the concise and reviewer explanation levels, and the reviewer packet.
  - ToolServices, complete for Phase 1: queries, and review approvals.
  - Cli: human-readable by default, `--json` output.
  - RapServer: error isolation, and stdout reserved for the protocol.
- **Suites added:** T5, T6.

### U6 — execution-core (`u6-execution-core`, library)

- **Description:** Phase 2 compilation to native code. [FR21–FR24]
- **Owns:**
  - Lowering: Core, Mono and RIR; Perceus reference counting and its optimizations.
  - CraneliftBackend, complete.
  - BuildDriver: dev mode, blocking on findings, linking, artifact cache.
  - RuntimeCore, complete: from-scratch allocator, reference counting, traps, profiling counters.
- **Boundaries:** the release backend and the full optimization pipeline belong to U12. Provenance records belong to U10.

### U7 — system-and-stdlib (`u7-system-and-stdlib`, library)

- **Description:** a usable language for real programs. [FR25, FR26, FR30, FR33–FR35]
- **Owns:**
  - SystemInterface: macOS and Linux operations, capabilities, the run manifest, and the no-sandbox native run with its warning.
  - StandardLibrary core: `std.core/col/text/bin/json/hash/st/fs/test/bench`, Untrusted and Secret types, and linear resource handles.
  - TestRunner.
  - Profiler.
- **Suites added:** T9, T10, T12.

### U8 — concurrency-and-system-std (`u8-concurrency-and-system-std`, library)

- **Description:** Phase 3 concurrency and the remaining system modules. [FR28, FR29, FR31]
- **Owns:**
  - SchedulerIo: tasks, work stealing, the io_uring and kqueue reactor, replay.
  - `std.task`, `std.par`, `std.proc`, `std.env`, `std.time`, `std.rand` and `std.log`.
- **Suites added:** T7.

### U9 — crypto-and-net (`u9-crypto-and-net`, library)

- **Description:** from-scratch cryptography and TLS 1.3, plus networking. [FR32, NFR8]
- **Owns:** CryptoTls (primitives, a sans-I/O TLS 1.3 state machine, certificate validation) and `std.net` (TCP, UDP, DNS, TLS).
- **Verification bar:** test vectors, interoperability with two independent TLS implementations, constant-time checks, and TLS parser fuzzing.
- **Notes:** Trust reuses its primitives (ADR-004).

### U10 — supply-chain (`u10-supply-chain`, service)

- **Description:** Phase 4. [FR38–FR46]
- **Owns:**
  - PackageManager: manifests, MVS, lockfile, content-addressed cache, vendoring, offline builds, publish with API-diff semver.
  - Trust: signing, transparency-log proofs, audit, provenance signing, and verify comparison.
  - RegistryServer: `rail registry serve`, the transparency log, and the advisory database.
  - BuildDriver reproducibility and unsigned provenance records.
  - `rail verify` orchestration in ToolServices (ADR-009).
  - FFI modules and `std.ffi`.
- **Suites added:** T3, T14.

### U11 — first-party-packages (`u11-first-party-packages`, library)

- **Description:** `rail/http` and `rail/zstd`, written in rAiL, versioned separately, and published to the local registry. [FR47]

### U12 — release-backend (`u12-release-backend`, library)

- **Description:** Phase 5 release performance. [FR48, FR49]
- **Owns:**
  - LlvmBackend (O2, ThinLTO, `musttail`).
  - The full fixed-order optimization pipeline in Lowering.
  - BuildDriver's release mode: dispatching release builds to LlvmBackend, and exposing it through `rail build --release` in ToolServices, Cli and RapServer.

### U13 — deep-analysis (`u13-deep-analysis`, library)

- **Description:** Phase 5 analysis and verification. [FR50–FR52]
- **Owns:**
  - LintEngine deep families, with their taint, uniqueness, termination, resource and escape analyses.
  - Verifier (bounded model checking with Bitwuzla).
  - Translator's remaining parts: the developer and audit explanation levels; the typed, control-flow, dependency and findings views; `rail equiv`.
  - Extensions to ToolServices, Cli and RapServer for `rail verify --bmc` / `verify.run`, `rail equiv` / `equiv.run`, and the new view and explanation levels.
- **Suites added:** T8.

### U14 — debugger (`u14-debugger`, library, Should)

- **Description:** a DAP-compatible `rail debug` with breakpoints on anchor paths. It may slip without affecting any other unit. [FR53]
- **Owns:** Debugger, plus its extensions to ToolServices, Cli and RapServer for `rail debug` / `debug.*`.

### U15 — wasm-sandbox (`u15-wasm-sandbox`, service, Should)

- **Description:** the Wasm component target and `rail run --sandbox`. It may slip; until it ships, native runs with capability enforcement are used (U7). [FR36, FR37]
- **Owns:**
  - WasmBackend.
  - SandboxHost (Wasmtime).
  - The execution limits and hash-chained audit log in SystemInterface.
  - BuildDriver's `wasm` mode.
  - Extensions to ToolServices, Cli and RapServer for `rail run --sandbox`. Once the sandbox is in place, this unit also removes the no-sandbox warning for agent-generated code.
- **Suites added:** T11.

### U16 — benchmark-corpus (`u16-benchmark-corpus`, spec)

- **Description:** 40 programs in four classes, each in rAiL, Rust, Go and OCaml. They are written under the same prompt and reviewed for fairness. The corpus lives outside the toolchain workspace. [FR54]
- **Placement:** it comes before syntax, because the T4 round trip (FR5.5) and every later corpus-based suite (T3, T7, T9, T10, T13) need it. The rAiL versions are written in canonical text straight from the spec. Each later unit's suites then validate them as that unit's capability lands (parsing in U3, checking in U4, running in U6 and U7). A corpus program that proves invalid against a landed unit is a defect, fixed with a reproducing test.
- **Registered in the harness:** corpus programs are registered as suite inputs in U2.

### U17 — measurement-tools (`u17-measurement-tools`, library)

- **Description:** measurement tooling outside the `rail` binary. [FR55, FR56]
- **Owns:**
  - PerfHarness (`rail-perf`). Only reference-machine results count.
  - ModelMeasurement: tokenizer counts across at least three model families, generation reliability, and evidence for the encoding gate. API keys stay out of the repository.
- **Suites added:** T13.

## Constraints that apply to every unit

- Tests come first ([practices] Testing Posture).
- At least 80% line coverage on Linux, and the floor can only rise.
- CI must be green on macOS and Linux before merge.
- The nine firm rules apply ([practices] discovered-rules).
- Every new crate needs a reason and the owner's approval. Cranelift, LLVM, Wasmtime and Bitwuzla are pre-approved.
- Units that touch a building block owned more fully by another unit must keep that block's interfaces as agreed in Contract Design.

## Assumptions & Open Questions

- [assumption] Complexity sizes are relative (S/M/L/XL). They are not time estimates.
- [assumption] The runtime-linked units (U6, U7, U8, U9) are tested both as Rust libraries and through compiled rAiL programs.
- Open question for Contract Design: the internal interfaces that cross unit boundaries. These are RIR (U6 → U12, U13, U15), the ToolServices operation set (U5 ↔ later units), the registry protocol (U10), and the run manifest (U7 ↔ U15).
