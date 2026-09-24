# Scope Definition & Prioritization — Questions

Upstream input: `ideation/intent-capture/intent-statement.md` (approved). It confirms the full 1.0 as the goal, one builder with AI agents, no fixed deadline, and a design-driven build order. The questions below turn that into an in/out boundary and a prioritized backlog.

## Q1. What is the first usable slice that should exist before anything else is built out?

The intent is the full 1.0 in design-driven order. A first usable slice gives an early point where rAiL can be tried with your agents, and a natural fallback if the full 1.0 takes longer than hoped.

A. Round-trip core: canonical text, binary and human forms with lossless conversion, IDs, hashes, `fmt` / `view` / `absorb` (the spec's Phase 0)
B. Agent loop: the above plus type and effect checking, lints and the agent protocol server with patches, so agents can write and edit checked rAiL (Phases 0–1)
C. Runs end to end: a thin path from canonical text through checking to a native binary for a small program, then widen each layer
D. No first slice: build the full 1.0 and treat it as usable only when complete
E. Not yet defined
X. Other (please specify)

[Answer]: B

## Q2. Which parts of the full 1.0 are must-have for this work, and which could be deferred if needed? (select all that are Should or Could, not Must)

The intent confirms the full 1.0. This question only marks what can slip if something has to, without removing it from scope.

A. LLVM release backend and the full optimization pipeline (Phase 5) — Should or Could
B. Bounded model checking verifier for `req` / `ens` contracts (Phase 5) — Should or Could
C. Wasm component target and sandbox host (Phase 3) — Should or Could
D. Nothing can slip: everything in the full 1.0 is Must
E. Not yet defined
X. Other (please specify)

[Answer]: C

## Q3. The spec's acceptance suites must pass on Linux, macOS and Windows. Which platforms are must-have?

A. All three are must-have, as the spec says
B. macOS and Linux are must-have; Windows can come later
C. One platform first (please say which); the others come later
D. Not yet defined
X. Other (please specify)

[Answer]: B

## Q4. The agent-reliability measure (spec §11.3: parse-valid and check-pass rates) has no numeric target yet. What target should count as success?

The spec's own gate only compares the canonical text with the human form (within 2 points on check-pass rate). Your success measure compares rAiL with human-oriented languages.

A. rAiL's check-pass rate is at least as high as Rust's on the same 1,000 generation tasks
B. rAiL's check-pass rate is higher than Rust's by a stated margin (please give it)
C. A fixed rate for rAiL alone (for example, 95% parse-valid and 90% check-pass; please give numbers)
D. Only the spec's §11.3 gate (canonical versus human form), with no comparison to other languages
E. Not yet defined
X. Other (please specify)

[Answer]: B (margin not yet given; see follow-up)

## Q5. Several acceptance suites need very large runs (for example 10⁶ generated modules in T1, 24 CPU-hours of fuzzing in T10, 1,000 model generation tasks per form in §11.3, and three reference machines in §11.1). How should those volumes be treated?

A. Run the full volumes as the spec states; they are part of the definition of done
B. Run scaled-down volumes in everyday CI, and the full volumes as a separate release check
C. Run scaled-down volumes only; the full volumes are out of this work
D. Not yet defined
X. Other (please specify)

[Answer]: B

## Q6. The performance targets (§11.2) and §11.3 need comparison programs in Rust, Go and OCaml, and calls to at least three model vendors' tokenizers and models. What is in this work?

A. All of it: write the 40-program corpus in all four languages, and run the model and tokenizer measurements
B. The corpus in rAiL and Rust only; Go and OCaml comparisons are out of this work
C. The corpus and benchmark harness are in; model and tokenizer measurements are run by you outside this work
D. Not yet defined
X. Other (please specify)

[Answer]: B

## Q7. When the design suggests several possible orders, what should decide which part is built first?

A. Risk first: the parts most likely to fail or force a redesign (for example token cost of the canonical text, reference-counting overhead)
B. Value first: whatever lets you use rAiL with your agents soonest
C. Dependency first: follow the spec's phase order where parts depend on each other
D. Walking skeleton first: a thin end-to-end path, then widen
E. Not yet defined
X. Other (please specify)

[Answer]: C

## Q8. The spec's §11.3 gate may switch the canonical text encoding from S-expressions to a human-like syntax before 1.0. When should that gate be decided?

A. Early, before most of the parser, printer and tooling work is built on the canonical text
B. When the spec says (at the end of Phase 1)
C. Only after the full toolchain exists
D. Not yet defined
X. Other (please specify)

[Answer]: C

## Q9. Q4 chose "higher than Rust's check-pass rate by a stated margin". What margin?

Measured on the same generation tasks, with the same models, for rAiL (canonical form) and Rust.

A. At least 5 percentage points higher
B. At least 10 percentage points higher
C. At least 20 percentage points higher
D. Not yet defined
X. Other (please specify)

[Answer]: B

## Q10. Q2 lets the Wasm sandbox slip, but the approved intent names "unsafe to run; built-in sandboxing and explicit, limited permissions" as a core problem. What exactly may slip?

The spec's capability checks (T12) and execution limits with audit logging (T11) can be enforced on native binaries too; only the Wasm component target and its host are Wasm-specific.

A. Only the Wasm component target and Wasmtime host may slip; capabilities, execution limits and audit logging stay Must on native binaries
B. The Wasm target and all sandboxing may slip; capabilities stay Must
C. All of it may slip, including capabilities and limits
D. Not yet defined
X. Other (please specify)

[Answer]: B

## Q11. Q6 puts Go and OCaml comparisons out of this work, but the approved intent measures success by the §11.2 targets "against Rust, Go and OCaml". Which holds?

A. Performance success is measured against Rust only; the Go and OCaml targets are dropped and the spec is updated to match
B. Rust comparison is in this work; Go and OCaml targets stay in the spec but are checked in later work
C. Keep Go and OCaml in this work after all
D. Not yet defined
X. Other (please specify)

[Answer]: C

## Consolidated Summary Confirmation

- First usable slice: the agent loop, meaning the spec's Phases 0–1 (three file forms, lossless conversion, IDs, hashes, `fmt` / `view` / `absorb`, type and effect checking, lints, and the agent protocol server with patches and merges) comes before anything else is built out (Q1).
- In scope: the full 1.0 (from the approved intent). What may slip if needed: the Wasm component target, its Wasmtime host, and the sandbox limits and audit logging (T11). Capability enforcement (T12) stays Must. Everything else is Must (Q2, Q10).
- Platforms: macOS and Linux are must-have; Windows can come later, so the spec's "all three OSes" is updated to match (Q3).
- Agent-reliability target: rAiL's check-pass rate is at least 10 percentage points higher than Rust's on the same generation tasks and models (Q4, Q9).
- Test volumes: scaled-down volumes in everyday CI; the spec's full volumes run as a separate release check (Q5).
- Comparison corpus: the 40-program corpus in rAiL, Rust, Go and OCaml, plus the model and tokenizer measurements, are all in this work (Q11 supersedes Q6).
- Build order: where the design allows a choice, follow dependencies in the spec's phase order (Q7).
- Canonical-text gate (§11.3): decided only after the full toolchain exists, not at the end of Phase 1; the spec is updated to match (Q8).

Does this all look correct before I generate the artifact?

Looks correct
Request changes

[Answer]: Looks correct
