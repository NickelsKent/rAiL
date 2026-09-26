# Delivery Planning — Questions

This stage puts the 17 units into a build order. A **Bolt** is one build pass over a piece of the work, ending in something that runs and is tested; Construction works through the Bolts one after another. The unit dependencies (from Units Generation) limit which orders are possible; within those limits, the order is your call.

Already settled, so not asked again:

- **Walking skeleton first.** A walking skeleton is a thin slice that runs end to end, built first to prove the pieces connect. You sign it off before anything else is built.
- **Phase order.** After the skeleton, work follows the spec's phase order wherever parts depend on each other. [scope Q7]
- **First usable slice.** The agent loop (Phases 0–1) is the first usable slice.
- **The canonical-encoding gate.** It is decided only after the full toolchain exists.
- **One builder working serially.** You build with AI agents, one unit at a time, so Bolts never run in parallel.
- **No team plan.** No separate team plan was made, so every Bolt is built by the developer agent under your approval.

## Q1. How should the order be chosen where the dependencies leave a choice?

A. Follow the spec's phase order, with the skeleton first; no scoring model (recommended; matches your earlier choice)
B. Score each Bolt with a Weighted Shortest Job First model, which ranks work by value, urgency and risk reduction divided by size, and follow the scores
X. Other (please specify)

[Answer]: A

## Q2. How big should one Bolt be?

A. One unit per Bolt: 17 Bolts, each with its own done-criteria and sign-off (recommended; matches the unit-by-unit Construction setting)
B. Bundle small related units into one Bolt (for example acceptance harness with benchmark corpus, supply chain with first-party packages): about 13 Bolts
X. Other (please specify)

[Answer]: A

## Q3. Where do the two "may slip" units go?

These are the debugger and the Wasm sandbox.

A. At the very end, after every Must unit, so slipping them never delays anything else (recommended)
B. In their phase position: the sandbox with Phase 3, the debugger with Phase 2
X. Other (please specify)

[Answer]: A

## Q4. Which risks should be pulled earlier than phase order would put them? (select all that apply)

Each option is a move the dependencies allow.

A. Performance: build the release backend (LLVM and the full optimizer) right after the execution core, so the §11.2 speed targets are tested early rather than at the end
B. Model reliability: build the measurement tools right after the system and standard library unit, so token counts and check-pass rates against Rust are measured early. The encoding gate decision still waits for the full toolchain.
C. TLS from scratch: build crypto and networking immediately after concurrency. It already comes next, so this only confirms it isn't pushed later.
D. None: keep strict phase order after the skeleton
X. Other (please specify)

[Answer]: A, B, C

## Q5. What outside the project could hold a Bolt up?

The candidates found so far:

- model-vendor API access (measurement tools)
- your reference machines (release checks, performance evidence)
- GitHub-hosted macOS and Linux runners (every merge)
- two independent TLS implementations for interoperability tests (crypto and networking)
- the pre-approved Cranelift, LLVM, Wasmtime and Bitwuzla libraries

A. That list is complete (recommended)
B. There is more (please list in Other)
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Ordering:** the skeleton comes first, then the spec's phase order. There is no scoring model. [Q1]
- **Bolt size:** one unit per Bolt, giving 17 Bolts, each with its own done-criteria and sign-off. [Q2]
- **"May slip" units:** the debugger and the Wasm sandbox come last, after every Must unit. [Q3]
- **Risks pulled earlier** [Q4]:
  - The release backend (LLVM and the full optimizer) is built right after the execution core, so the speed targets are tested early.
  - The measurement tools are built right after the system and standard library unit, so token counts and check-pass rates against Rust are measured early. The encoding gate decision still waits for the full toolchain.
  - Crypto and networking are built immediately after concurrency.
- **Resulting Bolt order:** walking-skeleton → acceptance-harness → benchmark-corpus → syntax → checker-and-lints → agent-loop (first usable slice) → execution-core → release-backend → system-and-stdlib → measurement-tools → concurrency-and-system-std → crypto-and-net → supply-chain → first-party-packages → deep-analysis → debugger → wasm-sandbox. Every step respects the unit dependencies.
- **External dependencies**, confirmed complete [Q5]:
  - model-vendor API access
  - your reference machines
  - GitHub-hosted macOS and Linux runners
  - two independent TLS implementations for interoperability tests
  - the pre-approved Cranelift, LLVM, Wasmtime and Bitwuzla libraries

Does this all look correct before I generate the artifact?

Looks correct
Request changes

[Answer]: Looks correct
