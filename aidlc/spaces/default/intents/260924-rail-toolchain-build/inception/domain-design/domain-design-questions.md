# Domain Design — Questions

The spec's prototype architecture (grammar → parser/printer → query engine → checker → lowering → backends → runtime, with the agent protocol server on the side) already fixes most building blocks. These questions cover the places where more than one split is reasonable. Each one changes what the building blocks are, who owns which data, and how later work can be divided. Recommendations come first.

Already settled: one `rail` program built from one Cargo workspace of several crates; the command line and the agent protocol server are thin front ends over shared services; the runtime depends on nothing beyond the platform C library; TLS and the allocator are written from scratch; the benchmark corpus sits outside the toolchain workspace.

## Q1. Should the translator be its own building block, separate from the parsers and printers?

The three encodings (canonical text, binary, human form) are generated from one grammar and must round-trip exactly. The translator's other outputs (explanations, typed and control-flow views, semantic diff, `rail equiv`) need type and effect information, so they depend on the checker.

A. Two blocks: **Syntax** (grammar, the three codecs, IDs, hashes, source maps) and **Translator** (explanations, derived views, semantic diff, equivalence), which depends on Syntax and the checker (recommended; the round-trip core stays small and has no dependency on type checking)
B. One **Representation** block covering both
X. Other (please specify)

[Answer]: A

## Q2. Should patching and merging be their own building block, rather than part of the agent protocol server?

`rail patch` / `rail merge` on the command line and `patch.apply` over the protocol do the same work. Your practices say both front ends stay thin.

A. A separate **Edit Engine** (patch validation and application, three-way merge, conflict items) used by both front ends (recommended)
B. Patching lives in the agent protocol server; the command line calls into the server's code
X. Other (please specify)

[Answer]: A

## Q3. How finely should the runtime be split?

The runtime covers the allocator, reference counting, traps, the scheduler, the I/O reactor, the `std.sys` platform layer, and now TLS and its cryptography written from scratch. These parts change at different rates and carry very different risks.

A. Four blocks: **Runtime Core** (allocator, reference counting, traps); **Scheduler & I/O** (tasks, work stealing, reactor, replay recording); **System Interface** (the 64 `std.sys` operations per platform, capability checks at the OS boundary); **Crypto & TLS** (primitives and the TLS 1.3 protocol, with its own verification bar) (recommended)
B. Two blocks: **Runtime** (everything except TLS) and **Crypto & TLS**
C. One **Runtime** block
X. Other (please specify)

[Answer]: A

## Q4. Where does the local registry server live?

The spec says all tooling ships in the single `rail` binary. The registry server also holds the transparency log and serves the advisory database.

A. Inside `rail`, as a `rail registry serve` command, built as its own block (**Registry Server**) that the package client talks to only over its network protocol (recommended; keeps "one binary" true and still lets the client be tested against it end to end)
B. A separate program, outside the `rail` binary
X. Other (please specify)

[Answer]: A

## Q5. Are the measurement and acceptance tools part of the shipped toolchain?

This covers `rail-perf`, the tokenizer and generation-reliability harness, and the T1–T14 acceptance harness. None of them is needed by someone who just uses rAiL, but `rail bench` and `rail prof` are.

A. No: they are project tooling in the repository, outside the `rail` binary, using `rail` as a black box plus its JSON outputs; `rail bench` and `rail prof` stay in the binary (recommended)
B. Yes: ship them all inside `rail`
X. Other (please specify)

[Answer]: A

## Q6. What should the command line print by default?

The spec says every command has JSON output. Agents will mostly use the protocol server, but they also call the command line, and you read its output too.

A. Human-readable text by default, JSON with `--json`, and the same JSON schema as the protocol server (recommended)
B. JSON by default, human-readable with `--human`
C. JSON only
X. Other (please specify)

[Answer]: A

## Q7. Where do the lint rules live?

Phase 1 lints (`FMT TY FX DEAD UNU`) work on the checked tree. The Phase 5 analyses (taint, uniqueness, termination, resource tracking) run on the lowered, monomorphized code. Both must come out of `rail check` as one diagnostic stream.

A. One **Lint Engine** that owns the rule registry, overrides and the diagnostic stream; tree-level rules read the checker's output, and deep rules read the lowered code, so the Lint Engine depends on both the checker and lowering (recommended)
B. Tree-level lints inside the checker; a separate **Analyzer** for the deep rules; the checker merges both streams
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Syntax and Translator are separate blocks.** Syntax owns the grammar, the three codecs, IDs, hashes and source maps. Translator owns the explanations, the derived views, the semantic diff and equivalence, and depends on Syntax and the checker. [Q1]
- **A separate Edit Engine** handles patch validation and application, three-way merge and conflict items. The command line and the agent protocol server both use it. [Q2]
- **The runtime is four blocks:** Runtime Core (allocator, reference counting, traps); Scheduler & I/O (tasks, work stealing, reactor, replay recording); System Interface (the `std.sys` platform operations and capability checks at the OS boundary); Crypto & TLS (primitives and TLS 1.3, with their own verification bar). [Q3]
- **The Registry Server is its own block inside `rail`** (`rail registry serve`). The package client talks to it only over its network protocol. [Q4]
- **Measurement and acceptance tools are project tooling in the repository, outside the `rail` binary:** `rail-perf`, the tokenizer and reliability harness, and the acceptance harness. They drive `rail` through its JSON output. `rail bench` and `rail prof` stay in the binary. [Q5]
- **The command line prints human-readable text by default.** `--json` gives the same JSON schema as the agent protocol. [Q6]
- **One Lint Engine** owns the rule registry, overrides and the single diagnostic stream. Tree-level rules read the checker's output; deep rules read lowered code. [Q7]

Does this all look correct before I generate the artifact?

Looks correct
Request changes

[Answer]: Looks correct
