# Architecture Decisions — Domain Design

Decision records for the component catalogue in `components.md`. Status of every record: Accepted at this stage's approval. Date: 2026-09-25. Sources: `requirements.md` (FR/NFR IDs), `team-practices.md`, the rAiL spec, and `domain-design-questions.md` ([Q1]–[Q7]).

## ADR-001: Follow the spec's prototype pipeline for the compiler blocks

- **Context** — The spec's prototype architecture sets out the pipeline: shared grammar → parser/printer → defhash-keyed query engine → checker → lowering → backends → runtime, with the protocol server beside it. The requirements build on its phase order (C5) and its per-phase acceptance suites.
- **Decision** — The compiler is decomposed along that pipeline: Syntax, QueryEngine, TypeChecker, Lowering, then one block per backend (CraneliftBackend, LlvmBackend, WasmBackend). All three backends consume RIR only.
- **Consequences** — Positive: each phase's exit suite tests one block or a short chain of them, and backends can be added or slip on their own (WasmBackend is Should). Negative: more block boundaries to keep stable; RIR becomes a de facto internal contract that all three backends depend on.
- **Alternatives Rejected** — (1) One "Compiler" block holding everything from parsing to code generation: simpler at first, but it hides the boundaries the acceptance suites test and makes the one-crate-per-concern layout (Code Style) arbitrary. (2) A separate block per IR level (Core, Mono, RIR): finer than any requirement needs, and the three levels always change together inside one lowering pass sequence.

## ADR-002: Separate Syntax from Translator

- **Context** — The three encodings must round-trip byte for byte (FR3, FR5; T1, T2, T4) and are generated from one grammar (FR1). The translator's other outputs — explanations, typed and control-flow views, semantic diff, equivalence (FR19, FR52) — need types, findings and IR. [Q1]
- **Decision** — Syntax owns the grammar, the three codecs including `view`/`absorb`, IDs, hashes, source maps and the generated model aids (FR18). Translator owns every other human-facing projection and depends on Syntax, TypeChecker, LintEngine, Lowering and TestRunner.
- **Consequences** — Positive: the round-trip core depends on nothing and can be finished and proven in Phase 0; a type-system change cannot break T1/T2/T4. Negative: "the translator" in the spec now spans two blocks, so documentation must say which part lives where.
- **Alternatives Rejected** — One Representation block: fewer boundaries, but Phase 0 code would be coupled to Phase 1 and Phase 5 analyses.

## ADR-003: One shared service layer, with a separate Edit Engine

- **Context** — The command line and the protocol server expose the same tools (FR6, FR13). Practices require both to be thin front ends over one shared service layer, and one bad request must never crash the server (FR13.1). Patching and merging (FR15, FR16) are needed by both. [Q2]
- **Decision** — ToolServices implements each tool operation once and returns one JSON schema; Cli and RapServer only parse input and render output. Patch validation/application and three-way merge live in their own EditEngine block, used through ToolServices.
- **Consequences** — Positive: identical behaviour on both front ends; the agent-loop edit logic is testable without any front end (T6). Negative: ToolServices depends on many blocks and becomes the busiest boundary; it must stay orchestration-only.
- **Alternatives Rejected** — Patching inside RapServer, with Cli calling server code: couples the command line to protocol framing and breaks the thin-front-end practice. Each front end orchestrating on its own: duplicated logic that would drift.

## ADR-004: Split the runtime into four blocks; isolate Crypto & TLS

- **Context** — The runtime must depend only on the platform C library (NFR7.1). It includes a from-scratch allocator (FR24.1) and a from-scratch TLS 1.3 stack with its own verification bar (FR32, NFR8). Memory safety (T10), concurrency (T7) and capability enforcement (T12) are tested separately. [Q3]
- **Decision** — Four blocks: RuntimeCore (allocator, RC, traps), SchedulerIo (tasks, scheduler, reactor, replay), SystemInterface (platform operations, capabilities, run manifest, limits and audit log), CryptoTls (primitives and a sans-I/O TLS 1.3 state machine). StandardLibrary sits above all four. Trust reuses CryptoTls primitives (Ed25519, BLAKE3) on the compiler side.
- **Consequences** — Positive: each block has one verification bar; TLS can be reviewed and fuzzed on its own; reusing CryptoTls in Trust avoids extra crypto crates under the dependency rule. Negative: four internal interfaces to keep stable. Reuse also means a CryptoTls defect affects both runtime TLS and package signing, so its verification bar protects both.
- **Alternatives Rejected** — One Runtime block: mixes the riskiest code with the fastest-changing code. Two blocks (Runtime plus Crypto & TLS): still ties memory management to scheduler and platform churn.

## ADR-005: The registry server lives inside `rail`, behind its own protocol

- **Context** — The spec ships all tooling in one binary. The local registry must hold a working transparency log and serve an advisory database (FR42–FR44). A public registry is out of scope. [Q4]
- **Decision** — RegistryServer is its own block, started by `rail registry serve`. PackageManager and Trust reach it only over its network protocol, never by in-process calls.
- **Consequences** — Positive: one binary; end-to-end tests exercise the real protocol; hosting it publicly later would need no client change. Negative: the binary carries server code that most users never run (counts toward the 60 MB limit).
- **Alternatives Rejected** — A separate registry program: a second artifact to build, sign and release, contrary to the single-binary rule.

## ADR-006: Measurement and acceptance tools are project tooling, not part of `rail`

- **Context** — `rail-perf` (FR55), model measurements (FR56) and the acceptance harness (FR57) are needed to prove the product, not to use it. Model measurements need vendor API keys, which must stay out of the product and the repository. `rail bench` and `rail prof` are product features (FR26). [Q5]
- **Decision** — PerfHarness, ModelMeasurement, AcceptanceHarness and BenchmarkCorpus live in the repository outside the toolchain workspace and drive `rail` as a black box through its JSON output and the protocol.
- **Consequences** — Positive: the binary stays smaller; black-box testing exercises the real interfaces; vendor access never enters the product. Negative: the tools depend on the JSON schemas staying stable, so schema changes need matching tool changes.
- **Alternatives Rejected** — Shipping them inside `rail`: larger binary, vendor dependencies in the product, and white-box shortcuts that would weaken what the suites prove.

## ADR-007: Human-readable command-line output by default, JSON on request

- **Context** — Every tool has JSON output (spec §9). Agents mostly use the protocol server; the owner reads command-line output. [Q6]
- **Decision** — Cli prints human-readable text by default; `--json` prints exactly the protocol's result schema.
- **Consequences** — Positive: readable for the owner; agents get the protocol schema on both interfaces. Negative: two renderers per command, and the human renderer must never carry information the JSON lacks.
- **Alternatives Rejected** — JSON by default: worse for the owner at the terminal. JSON only: no readable output at all.

## ADR-008: One Lint Engine owns all rules and the single diagnostic stream

- **Context** — `rail check` must emit parse, type and lint results as one stream with permanent rule IDs (FR11, FR11.3). Phase 1 rules run on the checked tree; Phase 5 analyses run on lowered code (FR50). [Q7]
- **Decision** — LintEngine owns the rule registry, overrides (FR12) and blocking policy. It depends on TypeChecker for tree-level rules and on Lowering for deep analyses.
- **Consequences** — Positive: one registry, one override policy, one stream. Negative: `rail check` must run lowering to Mono IR for the deep rules, which counts against the 50 ms incremental-check target. Per-function budgets and the query cache (FR7, FR11.5) are the mitigation.
- **Alternatives Rejected** — Tree-level lints inside TypeChecker plus a separate Analyzer: two registries and a merge point, which puts the one-stream guarantee at risk.

## ADR-009: Break the verify cycle through the service layer

- **Context** — `rail verify` rebuilds an artifact and compares bytes (FR45). Artifacts carry signed provenance. If Trust called BuildDriver to rebuild while BuildDriver called Trust to sign (and PackageManager called Trust), the blocks would form a cycle.
- **Decision** — BuildDriver writes an unsigned ProvenanceRecord. Trust signs and verifies records but never builds. ToolServices orchestrates `rail verify`: it rebuilds through BuildDriver, then asks Trust to compare and check signatures.
- **Consequences** — Positive: the dependency graph stays acyclic; Trust can be tested without a compiler. Negative: verify logic is split across two blocks, coordinated in ToolServices.
- **Alternatives Rejected** — Trust calling BuildDriver directly: a cycle through PackageManager. A combined Build-and-Trust block: mixes the fastest-changing block with the most security-sensitive one.

## ADR-010: Effect bounds reach the checker as workspace inputs

- **Context** — Each dependency's effect bound must be enforced against its exported signatures (FR38), and the checker also extracts the public API that PackageManager diffs on publish (FR40).
- **Decision** — PackageManager supplies resolved effect bounds into the QueryEngine workspace; TypeChecker enforces them without calling the package system. PackageManager depends on TypeChecker, never the reverse.
- **Consequences** — Positive: the checker has no dependency on the package system and can run standalone (Phase 1, before Phase 4 exists). Negative: a workspace without bounds must default to "no dependencies" rather than "unbounded".
- **Alternatives Rejected** — TypeChecker querying PackageManager: a cycle, and Phase 1 would depend on Phase 4 code.

## ADR-011: One capability model for native and sandboxed runs

- **Context** — Capability enforcement (T12) is Must. The Wasm sandbox may slip, so native runs enforce capabilities and warn that no sandbox is active (FR35). When the sandbox exists it must enforce the same grants (FR36), with limits and an audit log (FR37).
- **Decision** — SystemInterface owns capabilities, the run manifest, limits and the audit log. Native runs and SandboxHost both enforce grants through it. ToolServices prints the no-sandbox warning for native runs.
- **Consequences** — Positive: one enforcement point, so a native run and a sandboxed run behave identically for the same manifest; FR35 needs no throw-away code. Negative: SandboxHost is coupled to SystemInterface's interfaces.
- **Alternatives Rejected** — Separate capability logic in SandboxHost: two implementations of the rule that is never traded away (NFR5), and native runs would enforce it only as an afterthought.
