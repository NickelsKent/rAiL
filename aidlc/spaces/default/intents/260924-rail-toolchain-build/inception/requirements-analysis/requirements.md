# Requirements — rAiL Toolchain 1.0

## Sources

Source tags used throughout:

- `[desc]` — Initial description: "Build rAiL to the language spec catalogued at aidlc/spaces/default/knowledge/documents/rail-language-spec.md (document id 01a0d434-5009-77de-bde3-ca5af460eed1)".
- `[spec §n]` — the rAiL language specification, section n (T-numbers are its §12 acceptance suites; R-numbers its reference specification).
- `[intent]` — `ideation/intent-capture/intent-statement.md` (approved).
- `[scope]` — `ideation/scope-definition/scope-document.md` and `intent-backlog.md` (approved; `IB-nn` are backlog items).
- `[brief]` — `ideation/approval-handoff/initiative-brief.md` (approved).
- `[practices]` — `inception/practices-discovery/team-practices.md` and `discovered-rules.md` (affirmed).
- `[Q<n>]` — answers in `requirements-analysis-questions.md` (this stage), confirmed in its consolidated summary.

Priority uses MoSCoW as set in Ideation: everything in the full 1.0 is **Must** unless marked otherwise. [scope] "Phase" is the spec roadmap phase in which the requirement is first delivered; build order follows the spec's phase order, with the walking skeleton as the one agreed exception. [scope] [practices]

## Intent Analysis

The owner wants a working rAiL 1.0 toolchain: a single `rail` binary that lets AI agents write, check, edit, build, test and run programs in a language designed for them, and that measures whether that design actually works better for agents than human-oriented languages. [intent]

Four goals drive every requirement below: [intent]

1. **Unambiguous agent code.** One canonical spelling per program, stable addresses for edits, and structured diagnostics, so agent output diffs, merges and reviews cleanly.
2. **Safe to run.** No ambient authority; every file, network, process and environment access flows from an explicit capability; memory safety without a garbage collector.
3. **Evidence.** Acceptance suites, a benchmark corpus and model measurements that show whether the approach works, judged against Rust, Go and OCaml.
4. **Speed.** Native performance close to Rust.

The first usable slice is the agent loop (Phases 0–1): agents can write, check, lint, patch and review canonical rAiL through the agent protocol server. [scope] It must be usable on its own, because it is also the fallback if the full 1.0 takes longer than hoped. [brief]

Request type: new product (greenfield), system-wide, complex. Depth: Standard (the spec supplies most of the detail).

## Functional Requirements

Each requirement lists its verification: an acceptance suite (T1–T14), or a named test where no suite applies.

### Representation and translation (Phase 0)

**FR1 — Shared grammar.** The toolchain shall define the canonical text, canonical binary and human form from one shared grammar definition, from which the parsers and printers for all three are generated. [spec §8.4, conclusion] [scope IB-01]
- FR1.1 The grammar definition is the single source for parsers, printers, the grammar card (FR18.1) and the constrained-decoding grammar (FR18.2). [spec conclusion] [Q9]
- Verify: T1, T4; a build check fails if any generated artifact is out of date with the grammar. [practices]

**FR2 — Canonical text (`.rlc`).** The toolchain shall parse and print canonical text exactly as defined by the form set, grammar and formatting rules. [spec §2.2, §2.3, §2.5, R1–R4] [scope IB-02]
- FR2.1 Accept only the 18 core form heads plus `ext`, `req` and `ens`; no other keywords. [spec §2.2]
- FR2.2 Enforce the deterministic layout: UTF-8 NFC, LF, final LF, no BOM, single spaces, one item per line, fixed item order and sorting. [spec §2.5]
- FR2.3 Enforce each of the ten unique-spelling rules as a parse-time `FMT` error with a deterministic autofix that yields the canonical form. [spec §2.7]
- FR2.4 Emit literals in the canonical spelling (shortest round-trip floats, fixed string escapes). [spec §2.5]
- FR2.5 A truncated or corrupted input fails at a precise, located point. [spec §1.5]
- Verify: T1 (identical trees and diagnostics across runs and OSes; every mutation re-encodes canonically or is rejected with a located error); T2 (100% of non-canonical variants rejected with the right `FMT` ID and autofix equal to the canonical form).

**FR3 — Canonical binary (`.rlb`).** The toolchain shall encode and decode the canonical tree in the binary format (magic, version, fixed section order, opcode stream with LEB128 operands and string-table indices), and text ↔ binary conversion shall be a bijection. [spec §2.5, R9] [scope IB-03]
- Verify: T4 (text ↔ binary identical); `rail fmt --check` verifies both directions. [spec §2.5]

**FR4 — Identifiers and hashes.** The toolchain shall assign, preserve and use definition IDs, anchor paths and content hashes. [spec §2.6, R10] [scope IB-04]
- FR4.1 Definition IDs: 30 random bits in lowercase Crockford base32, assigned on creation, never reused within a module, preserved across renames and moves.
- FR4.2 Anchor paths address nodes inside a definition by binding name or child path; shadowing is an error, so name anchors are stable.
- FR4.3 `defhash` = BLAKE3 of the definition's binary tree with locals as de Bruijn indices, excluding IDs and metadata; `modhash` = BLAKE3 of the sorted definition table.
- Verify: T6 (untouched definitions keep identical `defhash` after edits); unit tests for hash invariance under renaming of locals and metadata changes.

**FR5 — Human form and translator core (`.rlh`, `view`, `absorb`).** The toolchain shall render canonical modules as formatter-canonical human text and parse such text back. [spec §2.4, §2.10, §8.1, §8.4, R11] [scope IB-05]
- FR5.1 `view` never fails on a valid canonical module; `absorb(view(c)) == c` byte for byte.
- FR5.2 `view(absorb(h)) == h` for every formatter-canonical human text; non-canonical human text is rejected with `FMT` diagnostics and an autofix.
- FR5.3 Rendering rules: two-space layout, fixed precedence table, Boolean matches as `if`, pipelines for first-argument chains of depth three or more, named record literals.
- FR5.4 Definition IDs appear as trailing `--#id` markers when requested (default for editable output); unmarked edited definitions match by name; unmatched ones get new IDs.
- FR5.5 After every `absorb`, the `defhash` of untouched definitions is compared with the previous version; any difference aborts with an internal-error diagnostic. [spec §8.7]
- Verify: T4 over the corpus, the standard library and 10⁵ generated modules at full volume (CI volume recorded per [practices]).

**FR6 — Phase 0 command line.** The `rail` binary shall provide `parse`, `fmt` (with `--check`), `view` and `absorb`, each with JSON output. [spec §9.1] [scope IB-06]
- Verify: CLI integration tests; T1, T2, T4 run through these commands.

### Checking and diagnostics (Phase 1)

**FR7 — Incremental query engine.** The toolchain shall cache every compiler phase keyed by `defhash`, so re-checking after an edit touches only the edited definitions and their dependents. [spec §9.3, conclusion] [scope IB-07]
- Verify: NFR2 incremental-check target; tests that an edit to one definition does not recompute unrelated definitions.

**FR8 — Types.** The checker shall implement Hindley–Milner inference with top-level generalization only, the primitive types, ADTs, single-parameter coherent traits with no overlap, and explicit numeric conversions. [spec §3.1, §3.5, §3.7, R5] [scope IB-08]
- FR8.1 Signatures are required on exported functions and trait methods and inferred elsewhere.
- FR8.2 `(! e)` propagation follows R5, applying `From` only when exactly one implementation matches; ignoring a `Result` is error `TY020`. [spec §7.1, §7.2]
- Verify: T9 (std and corpus tests); checker unit tests per rule with at least one accepting and two rejecting cases. [practices]

**FR9 — Effects.** The checker shall infer and check effect rows over the 12 effect labels plus at most one row variable, with subsumption only by row extension. [spec §3.3, §3.8, R5, R7]
- FR9.1 A function using an effect absent from its declared signature is error `FX001` with an autofix that widens the signature and raises an `A` finding.
- Verify: checker tests; T8 for the `FX` family.

**FR10 — Pattern checking.** Matches shall be exhaustive and every arm reachable; both are errors. [spec §3.6, R4]
- Verify: checker tests; T8 for `DEAD001`.

**FR11 — Lint framework and diagnostics.** `rail check` shall run parsing, typing, effect checking and linting as one pass producing one structured diagnostic stream; no build can skip it. [spec §1.1, §6, R12] [scope IB-09]
- FR11.1 Every diagnostic follows the §6.4 schema: rule, level, location (module, definition, anchor path, byte span), message, remedy as a structured patch, autofix with determinism and semantics flags, confidence, impact axis.
- FR11.2 Levels `E S W O A` with the blocking behavior of §6.1; heuristic rules (confidence below 1.0) never block builds.
- FR11.3 Rule IDs are permanent, defined once in one registry, and never reused. [spec §6.2] [practices]
- FR11.4 Phase 1 delivers the `FMT`, `TY`, `FX`, `DEAD` and `UNU` families. [spec roadmap]
- FR11.5 Analyses have per-function budgets that degrade to a `W` "analysis incomplete" finding rather than timing out. [spec §6.3]
- Verify: T8 (partial in Phase 1, full in Phase 5); schema conformance tests on every emitted diagnostic.

**FR12 — Overrides.** Lint overrides shall be canonical `allow` items naming exactly one rule and one target, with a required reason and optional expiry; `E` rules cannot be overridden; `S-crit` overrides need a signed approval in `rail.review`; `rail check --overrides` lists active overrides with age. [spec §6.5]
- Verify: tests for each override rule, including expiry turning an override back into a finding.

### Agent loop and tooling (Phase 1)

**FR13 — Agent protocol (RAP).** The `rail` binary shall serve the rAiL Agent Protocol: JSON-RPC 2.0 over standard input and output, keeping a warm incremental compiler in memory, exposing the RAP method for every tool in the §9.1 table that exists at that phase. [spec §9, §9.1] [scope IB-10]
- FR13.1 One malformed or failing request never terminates the server; it returns a structured error. [practices]
- FR13.2 No component writes anything to standard output except protocol messages while RAP is running. [practices]
- Verify: RAP integration tests including malformed-request and internal-failure cases.

**FR14 — Addressing.** Agents shall address code as `module#defid/anchor-path`; line and column positions are accepted only as a fallback and converted to anchors immediately; `tree.get` accepts `depth` and `focus` to return only the needed subtree. [spec §9.2]
- Verify: RAP tests for each addressing form and for line/column conversion.

**FR15 — Structured patches.** `rail patch` / `patch.apply` shall apply canonical-text patches with operations `set ins add del ren mov meta`, pinned to a `base` hash, atomically, reporting failures by operation index and returning new `defhash` values and the diagnostic delta for touched definitions. [spec §9.3, §9.4, R13]
- FR15.1 If the `base` hash differs from the current module, the call fails fast and returns the intervening patch. [spec §9.5]
- Verify: T6 (≥ 99% of 500 scripted edit tasks apply; untouched definitions keep identical `defhash`).

**FR16 — Merge.** `rail merge` shall perform a three-way merge on the tree per definition, merging disjoint anchor paths within a definition and reporting only overlapping node edits as structured `(conflict …)` items, never text markers. [spec §9.5]
- Verify: T6 (concurrent disjoint patches merge with zero conflicts).

**FR17 — Source maps.** The toolchain shall produce `.rlmap` source maps in canonical text, total in both directions, depending only on the canonical tree. [spec §8.3] [scope IB-10]
- Verify: T5 (after 10⁴ random single-definition patches, map lines of untouched definitions are unchanged except byte offsets; every human span maps to exactly one node).

**FR18 — Model-writing aids.** Phase 1 shall deliver, generated from the shared grammar: [Q9] [spec risk 2]
- FR18.1 A grammar card suitable for inclusion in model prompts.
- FR18.2 A constrained-decoding grammar for canonical text.
- Verify: generated artifacts are checked for being up to date in CI; every module in the T1 valid set is accepted by the constrained-decoding grammar.

**FR19 — Review translator views, Phase 1 part.** Phase 1 shall deliver: [Q1] [spec §8.2, §8.6, §9.7]
- FR19.1 `rail diff --human` / `diff.run`: definitions added, removed, renamed, moved or changed, shown side by side in human form, with deltas in effects, capabilities, dependencies, findings and exported signatures.
- FR19.2 The `concise` and `reviewer` explanation levels, generated from templates driven by the typed tree; any model-written narrative is labeled non-authoritative.
- FR19.3 The reviewer packet at the end of the agent loop; approval records the reviewed `modhash`.
- Verify: golden-file tests on the §10 examples; tests that explanations mention only effects and calls present in the tree.

**FR20 — Queries.** `rail query` / `query.run` shall return callers, callees, users of an effect and definitions by type. [spec §9.1]
- Verify: query tests on the §10 examples and the standard library.

### Execution (Phase 2)

**FR21 — Lowering.** The compiler shall lower checked code through Core IR (typed, explicit evidence), Mono IR (monomorphized, closure-converted, decision trees, fused iterators) and RIR (SSA with explicit `inc`, `dec`, `reuse`, `drop`), with RIR serializable for caching. [spec §4.1] [scope IB-11]
- Verify: T9; IR-level golden tests.

**FR22 — Memory management.** The compiler shall insert reference counting per the Perceus algorithm with borrow inference, reuse analysis, drop specialization and fusion, and unboxed small values; counts are non-atomic unless a value crosses a task boundary. [spec §3.12, §4.4, R8]
- Verify: T10 (zero memory errors and leaks); NFR1 RC-overhead target.

**FR23 — Native backend and calls.** Dev builds shall use Cranelift, producing native binaries for x86-64 and AArch64, with guaranteed tail calls for all tail-position calls (self tail calls as jumps). [spec §3.10, §4.2, §4.5] [scope IB-12]
- Verify: T9 including dedicated tail-call tests per platform. [spec risk 6]

**FR24 — Runtime.** The runtime shall be a static library providing the allocator, reference counting, scheduler, I/O reactor, trap path and the `std.sys` interface, with the scheduler and reactor starting lazily. [spec §4.3, §4.9] [scope IB-13]
- FR24.1 The allocator is written from scratch (size classes in the style of mimalloc, used only as a design reference). [Q4]
- FR24.2 Traps (`abort`, overflow, out-of-bounds, division by zero, stack limit) end the current task; its scope receives `Err(TaskTrap)`; an unhandled trap in `main` exits with code 70 and a structured trace. [spec §7.3]
- Verify: T10; runtime unit tests; NFR7 dependency check on the runtime.

**FR25 — Core standard library.** Phase 2 shall deliver `std.core`, `std.col`, `std.text`, `std.bin`, `std.json`, `std.hash`, `std.st` and `std.fs`, plus `std.test` and `std.bench`. [spec §5.1, roadmap] [scope IB-14]
- Verify: T9 (100% std tests pass); T10 fuzzing of std parsers.

**FR26 — Tests, benchmarks and profiling.** [spec §9.1, §9.6] [Q2]
- FR26.1 `rail test` / `test.run` runs `tst` items, including property tests with shrinking and deterministic seeds, reporting counterexamples as canonical literals; contracts (`req`, `ens`) are checked at runtime in dev and tests.
- FR26.2 `rail bench` / `bench.run` reports median, p99, allocations and bytes with confidence intervals.
- FR26.3 `rail prof` / `prof.run` produces sampled CPU, allocation and reference-counting profiles keyed by definition ID. **Must.** [Q2]
- Verify: tool tests; the profiler's RC attribution is used to measure the RC-overhead target (NFR1).

**FR27 — Walking skeleton.** Before Phase 1 is complete, a one-function module shall be parsed, checked, served through RAP, compiled with Cranelift and run as a native binary on macOS and Linux, verified by one recorded command the owner approves. [practices]
- Verify: the recorded verification command, on both platforms.

### Concurrency and safety (Phase 3)

**FR28 — Structured concurrency.** The runtime shall provide green-thread tasks with growable stacks on an M:N work-stealing scheduler, structured scopes (tasks cannot outlive their scope; a failing task cancels siblings; the scope returns the first error by spawn order), channels and atoms, and blocking-style I/O over non-blocking OS interfaces (io_uring on Linux, kqueue on macOS). [spec §4.7, §5.1 `std.task`, `std.par`] [scope IB-15]
- Verify: T7; concurrency tests.

**FR29 — Deterministic concurrency and replay.** Programs without the `nd` effect produce the same observable output on every run and core count; `rail run --replay` records and replays `clk`, `rnd`, `net` and `nd` outcomes exactly. [spec §4.8] [scope IB-15]
- Verify: T7 (identical stdout, stderr, exit codes, written files and audit logs across runs and platforms).

**FR30 — Capabilities.** `main` shall receive a `Caps` record built by the runtime from the run manifest; nothing else can create capabilities; scoping functions only narrow them. Files open relative to a directory capability (no `..` escape, symlinks resolved and checked); `proc` capabilities list allowed executables and never invoke a shell; `env` capabilities list readable names. [spec §3.8, §7.5, R14] [scope IB-16]
- Verify: T12 (100% of out-of-scope operations denied with typed errors, including `..`, symlink and DNS-rebinding tricks; no side effect outside the grant). **Must.** [scope]

**FR31 — System standard library.** Phase 3 shall deliver `std.net` (TCP, UDP, DNS), `std.proc`, `std.env`, `std.time`, `std.rand`, `std.log` and `std.ffi`, all behind the capability model. [spec §5.1] [scope IB-16]
- Verify: T7, T12; module tests.

**FR32 — TLS 1.3.** `std.net` shall include a TLS 1.3 client and server, written from scratch including its cryptographic primitives (rustls used only as a design reference). [Q3] [Q4] [spec §5.1]
- FR32.1 Every cryptographic primitive passes its published test vectors. [Q10]
- FR32.2 Interoperability tests pass against at least two independent TLS implementations. [Q10]
- FR32.3 Secret-dependent code passes constant-time checks. [Q10]
- FR32.4 TLS record and handshake parsers are in the fuzzing set. [Q10]
- Verify: the four checks above; T10 fuzz run.

**FR33 — Untrusted input and secrets.** External data shall enter as `Untrusted[a]`; dangerous sinks require validated or `Trusted` values via `validate`; `Secret[a]` has no `Show`/`Json`, is zeroed on free, compares in constant time and is readable only through `secret.expose`. [spec §7.7, §7.8]
- Verify: type-checker tests; T8 for `SEC001` and `SEC010` (Phase 5).

**FR34 — Resource handles.** `File`, `Socket`, `Child` and `Listener` shall be linear handles consumed exactly once, with scoped helpers that close on every exit path including traps; `RES001` enforces this. [spec §7.4]
- Verify: T8 for `RES001`; tests that handles close on traps.

**FR35 — Running without the sandbox.** Until FR36 exists, `rail run` shall run code natively with capability enforcement, grant nothing the run manifest does not list, and print a clear warning that no sandbox is active. [Q8]
- Verify: tests that an empty manifest grants nothing and that the warning is printed.

**FR36 — Wasm sandbox. (Should — may slip.)** The compiler shall produce WebAssembly components, and `rail run --sandbox` shall execute them in an embedded Wasmtime host with only manifest-granted capabilities, fuel metering, a memory cap, a wall-clock deadline, a task cap and deterministic mode by default; once it exists it is the default for agent-generated code. [spec §4.2, §4.10] [scope IB-18]
- Verify: T13 sandbox cold-start target; T12 in the sandbox.

**FR37 — Execution limits and audit log. (Should — may slip.)** Run manifests shall set CPU fuel, memory, wall time, handles, tasks and output bytes (sandbox defaults 10 s, 512 MiB, 256 handles, 1,024 tasks), and the runtime shall write a hash-chained JSON-lines audit log of capability grants and uses (secrets redacted), traps and limit hits, attributed to definition IDs. [spec §7.9] [scope IB-17]
- Verify: T11.

### Supply chain (Phase 4)

**FR38 — Packages and manifests.** A package shall be a directory with `rail.pkg` and `rail.lock` in canonical text; module paths are file paths; module dependency cycles are errors; each dependency declares an effect bound that the compiler enforces against every exported function; `dev` dependencies are visible only to tests and benchmarks. [spec §5.2, §5.3] [scope IB-19]
- Verify: T14; package tests including an effect-bound violation.

**FR39 — Resolution and lockfile.** Resolution shall be minimal version selection; `rail.lock` records exact version, source, BLAKE3 archive hash, publisher key fingerprint and resolved effect bound, sorted. [spec §5.4] [scope IB-19]
- Verify: resolution tests; T3.

**FR40 — Semantic versioning.** `rail publish` shall diff the public API against the previous release and refuse a version bump smaller than the change requires; widening an exported effect row is breaking. [spec §5.4] [scope IB-19]
- Verify: publish tests for each change class.

**FR41 — Integrity, cache and offline builds.** Packages are canonical binary archives, hash-verified on download and on every cache read; the global cache is content-addressed; compiled artifacts are cached by `(defhash, compiler version, target, mode)`; `rail vendor` and `rail build --offline` work without network. [spec §5.5] [scope IB-19]
- Verify: T14.

**FR42 — Signing and transparency log.** Every registry package shall be signed with Ed25519 by its publisher and countersigned by a registry transparency log with inclusion proofs; `rail` verifies both. [spec §5.9, §7.11] [Q7] [scope IB-20]
- Verify: tests for valid, missing, forged and non-included signatures.

**FR43 — Local registry server.** The toolchain shall include a registry server that can be run locally, serving packages, the transparency log and the advisory database. Hosting a public registry is out of scope. [intent] [Q7] [scope IB-20]
- Verify: end-to-end publish, resolve, download and verify against a local registry.

**FR44 — Audit.** `rail audit` / `audit.run` shall check locked packages against an advisory database file in a documented format (served by the registry), flag yanked versions, typosquat-similar names and packages published less than 72 hours ago, and report the capability map; new dependencies and widened effect bounds raise `DEP010` findings. [spec §5.9] [Q7]
- Verify: audit tests against a fixture advisory database.

**FR45 — Reproducible builds and provenance.** For a fixed compiler version, target, mode and lockfile, `rail build` shall produce byte-identical artifacts with no timestamps, absolute paths, hostnames or random seeds; artifacts carry a signed provenance record; `rail verify` rebuilds and compares bytes. [spec §7.10, §7.11] [scope IB-21] [practices]
- Verify: T3.

**FR46 — Foreign-function interface.** Foreign code shall be reachable only from `(ffi-module)` modules declaring `ext` items with `std.ffi` types; the `ffi` effect propagates until discharged by a reviewed `SEC101` override; foreign libraries are listed with version and hash and statically linked. [spec §5.8]
- Verify: tests that `ffi` cannot be used outside an ffi-module and that discharge requires the override.

**FR47 — First-party packages.** This work shall deliver `rail/http` and `rail/zstd` as separately versioned first-party packages built with the package system. [Q6] [spec §5.1]
- Verify: each package's own tests; published to and resolved from the local registry.

### Performance, analysis and verification (Phase 5)

**FR48 — Release backend.** Release builds shall use LLVM at O2 with ThinLTO across modules, with guaranteed tail calls (`musttail`). [spec §4.2, risk 6] [scope IB-22]
- Verify: T9 under release mode; T13.

**FR49 — Optimization pipeline.** The compiler shall run the eight passes of §4.6 in the fixed order, deterministically (fixed iteration caps, no dependence on hash-map order or time). [spec §4.6] [scope IB-22]
- Verify: T3 (determinism); T13.

**FR50 — Remaining lint families.** Phase 5 shall complete the §6.2 v1 catalog, including `SEC`, `PERF`, `REC`, `CON`, `RES`, `PLT`, `ND`, `DEP`, `PUR` and `AGT`, with the analyses of §6.3 (taint, uniqueness, termination, resource linearity, region escape). [spec §6.2, §6.3] [scope IB-23]
- Verify: T8 (recall ≥ 95% for sound rules and 100% for `E`, ≥ 80% for heuristic rules; false-positive rate ≤ 2% on clean controls).

**FR51 — Bounded model checking.** `rail verify --bmc` shall check `req` / `ens` contracts statically over RIR for pure integer and ADT code using a bundled Bitwuzla solver; `par.fold` associativity and `Ord` laws are checked by generated property tests. [spec §9.6] [scope IB-24]
- Verify: verifier tests with known-valid and known-violated contracts.

**FR52 — Remaining translator features.** Phase 5 shall deliver the `developer` and `audit` explanation levels, the typed, control-flow, dependency-summary and findings views, and `rail equiv` / `equiv.run` reporting canonical, hash or tested equivalence (10,000 generated cases by default, reported as evidence, not proof). [Q1] [spec §8.1, §8.2, §8.5]
- Verify: golden-file tests; equivalence tests with known-equal and known-different pairs.

**FR53 — Debugger. (Should — may slip.)** `rail debug` shall provide a Debug Adapter Protocol compatible debugger with breakpoints on anchor paths and value printing in human form. [Q2] [spec §9.1]
- Verify: DAP conformance tests on the §10 examples.

### Measurement

**FR54 — Benchmark corpus.** The work shall include 40 programs in four classes (pure data transformation, parsing and text, I/O services with typed errors, data-parallel compute), each written in rAiL, Rust, Go and OCaml under the same prompt and reviewed for fairness. [spec §11.1] [scope IB-25]
- Verify: T9 (differential testing against the Rust version on 10⁴ generated inputs, zero differences).

**FR55 — Performance harness.** `rail-perf` shall run the corpus with 30 runs after 5 warm-ups, reporting median and p95 with 95% bootstrap intervals; a target counts as met when the upper bound meets it. Only results from the owner's reference machines count. [spec §11.1] [practices] [scope IB-25]
- Verify: T13.

**FR56 — Model measurements and the encoding gate.** The work shall tokenize every corpus program in each form with the official tools of at least three model families, report tokens per program, definition and patch, and measure parse-valid and check-pass rates on 1,000 generation tasks per form; the §11.3 canonical-encoding gate is decided after the full toolchain exists. [spec §11.3] [scope] [scope IB-26]
- Verify: the measurement report; gate decision recorded and the spec updated.

### Acceptance and project upkeep

**FR57 — Acceptance harness.** All 14 acceptance suites shall be automated. Everyday CI runs them at scaled-down volumes whose value and seed policy are recorded in the repository; the release check runs the spec's full volumes. [spec §12] [scope] [practices] [scope IB-27]
- Verify: each suite's recorded CI volume and full-volume release run; reducing a recorded volume counts as lowering a pass criterion. [practices]

**FR58 — Spec maintenance.** Every decision in this workflow that changes the spec shall be written into the spec in the same change; the authoritative spec lives in `spec/` in the repository, and the knowledge-folder copy is synced from it. [intent] [practices] [scope IB-28]
- FR58.1 Pending spec updates from decisions so far: Windows not required for this work (§11.1, §12); scaled CI volumes (§12); §11.3 gate timing (roadmap); agent-reliability target (§11.3); Wasm target, host and T11 may slip (roadmap Phase 3); runtime zero dependencies with approved compiler crates (§11.2 dependency count, T14); translator feature phases (roadmap); allocator and TLS written from scratch (§4.3, §5.1); debugger may slip (§9.1). [scope] [practices] [Q1] [Q2] [Q4]
- Verify: a review check that every recorded decision has a matching spec change.

**FR59 — Windows. (Could — later.)** Windows support (IOCP, Windows cold-start target, `dumpbin` check, Windows reference machine) is deferred beyond this work's must-haves. [scope IB-29]

## Non-Functional Requirements

**NFR1 — Performance targets.** The release toolchain shall meet every §11.2 target on the macOS AArch64 and Linux x86-64 reference machines, measured by `rail-perf` (FR55): [spec §11.2] [scope]

| Metric | Target |
|--------|--------|
| Clean dev build, 10 K canonical lines, 16 cores | ≤ 1.5 s |
| Clean release build, same | ≤ 10 s |
| Incremental check (one-definition patch, 100 K-line package, to diagnostics) | ≤ 50 ms p95 |
| Incremental build (same patch, to runnable dev binary) | ≤ 300 ms p95 |
| Native cold start, hello-world | ≤ 5 ms |
| Sandbox cold start, cached component (only if FR36 ships) | ≤ 15 ms |
| Execution time, corpus geometric mean, release | ≤ 1.3× Rust; ≤ 1.0× Go; ≤ 0.9× OCaml |
| Peak memory, geometric mean max RSS | ≤ 1.3× Rust; ≤ 0.8× Go |
| Binary size, hello-world / corpus median, stripped | ≤ 400 KB / ≤ 1.5 MB |
| Allocation rate, corpus geometric mean | ≤ 1.5× Rust |
| Reference-counting overhead, profiler-attributed | ≤ 8% median |
| Parallel speedup, data-parallel class, 16 cores | ≥ 12× |
| Canonical binary size vs gzip of Rust source | ≤ 0.6× |
| Model tokens, canonical text | ≤ 1.0× human form; ≤ 0.7× Rust |
| Toolchain size, single `rail` binary with runtime and std | ≤ 60 MB |

- Verify: T13.

**NFR2 — Agent reliability.** Canonical rAiL's check-pass rate shall be at least 10 percentage points higher than Rust's on the same generation tasks and models. [scope] [intent]
- Verify: FR56 measurement report.

**NFR3 — Determinism.** Parsing, checking, formatting, builds and default execution shall be deterministic across runs, directories, users, clocks, core counts and the two required platforms. [spec §1.4, §4.8, §7.10] [practices]
- Verify: T1, T3, T7.

**NFR4 — Memory safety.** Safe rAiL shall have no null, dangling references, uninitialized memory or data races, with bounds-checked indexing and checked arithmetic; unsafety exists only in `ffi-module`s and in the runtime's allowed Rust `unsafe` code. [spec §7.6] [practices]
- Verify: T10 (zero memory errors, zero leaks, zero fuzz crashes other than typed errors over 24 CPU-hours at full volume).

**NFR5 — Capability safety.** No code shall touch the file system, network, processes, clock, randomness or environment except through capabilities granted by its caller; memory and capability safety are never traded for speed. [spec §1.1, §1.4]
- Verify: T12; T11 if FR37 ships.

**NFR6 — Portability.** Observable behavior shall be identical on macOS and Linux; platform differences surface as typed values (`Unsupported`), never as divergent semantics; only 64-bit targets. [spec §1.4, §4.4, §4.9] [scope]
- Verify: T7.

**NFR7 — Dependencies.** [practices] [Q5] [spec §11.2, T14]
- NFR7.1 The runtime depends on nothing beyond the platform C library.
- NFR7.2 The compiler's Rust dependencies are an approved list, each with a recorded reason; Cranelift, LLVM, Wasmtime and Bitwuzla are pre-approved. [Q5]
- NFR7.3 The toolchain resolves zero external rAiL packages; hello-world has zero transitive packages; `ldd` / `otool` show only OS system libraries.
- Verify: T14; a CI check that the runtime's dependency tree is empty. [practices]

**NFR8 — TLS assurance.** The TLS stack shall meet the verification bar of FR32.1–FR32.4 before 1.0. [Q10]

**NFR9 — Code quality.** Rust code shall have at least 80% line coverage measured on Linux (excluding the corpus, test harnesses and grammar-generated code), with a floor that only rises, and shall be developed test-first. [practices]
- Verify: CI coverage gate.

**NFR10 — Supply-chain hygiene of the toolchain itself.** CI shall run the affirmed security checks (crate advisories, bans, sources and licenses; secret scanning; lockfile-exact builds; fuzzing; pinned CI actions), and every release shall ship checksums, a build record and a dependency list. [practices]
- Verify: CI configuration review; release checklist.

## Constraints

- **C1** The compiler and tools are written in Rust as one Cargo workspace of several crates producing one `rail` binary; Rust 2024; one pinned stable toolchain plus a pinned nightly lane only for fuzzing, sanitizers and `miri`. [spec conclusion] [practices]
- **C2** macOS and Linux are required; Windows is later. [scope]
- **C3** One builder (the owner) with AI agents, no fixed deadline; contributors may join later, so the design should make joining easy. [intent]
- **C4** No build-time code execution anywhere in rAiL's package system: no build scripts, procedural macros, install hooks or network access during builds. [spec §1.2, §5.9]
- **C5** Build order follows the spec's phase order where parts depend on each other; the walking skeleton is the one exception. [scope] [practices]
- **C6** Firm rules apply to all work: never lower a pass criterion, recorded CI volume or coverage floor; no secrets in the repository; spec updated with any decision that changes it; no timestamps, absolute paths, hostnames or random seeds in compiler output; CI green on macOS and Linux before merge; release tags never move; every defect fix comes with a reproducing test; every new Rust dependency has a reason and approval; nothing published to a public registry. [practices]
- **C7** License: MIT OR Apache-2.0. [practices]
- **C8** Explicit non-goals of the spec stay out: dynamic typing, reflection, `eval`, macros, user-defined syntax, resumable effect handlers, higher-kinded and dependent types, tracing GC, operator overloading beyond the fixed set, build-time code execution, a REPL. [spec §1.2, §1.3]

## Assumptions & Open Questions

- [assumption] The owner's reference machines match the spec's §11.1 profiles for macOS AArch64 (10 cores) and Linux x86-64 (16 cores); if not, T13 targets need re-baselining. [brief]
- [assumption] Model access covers at least three model families with official token-counting tools. [brief]
- [assumption] The toolchain size target (≤ 60 MB) holds with LLVM, Wasmtime and Bitwuzla linked into one binary; to be checked in design, since these three are large.
- [assumption] The spec's Windows-specific items (IOCP, the Windows cold-start target, `dumpbin`, the Windows reference machine) are deferred with Windows itself. [scope]
- Open question for design: whether `rail/zstd` is written in rAiL or needs the FFI path; this requirement assumes pure rAiL, which keeps the runtime dependency-free.
- Open question for design: the effect of the §11.3 gate on the grammar card and constrained-decoding grammar; both are generated from the shared grammar, so a later encoding switch regenerates them.
- Open question for design: exact CI and release volumes and seed policies for each suite (recorded in the repository per FR57).

## Out of Scope

- Hosting a public package registry. [intent]
- Deployment and operations. [intent]
- Self-hosting the compiler. [spec conclusion]
- Windows as a must-have platform for this work (FR59 is Could). [scope]
- First-party packages other than `rail/http` and `rail/zstd` (for example databases). [Q6]
