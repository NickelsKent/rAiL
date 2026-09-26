# Requirements Analysis — Questions

The rAiL spec already defines most of the requirements in detail, and Ideation and Practices Discovery settled scope, priorities, platforms and working practices. These questions cover only the gaps: parts of the spec with no roadmap phase, parts where your earlier decisions interact with the spec, and dependencies the spec names that your new firm rules now cover.

Already settled (not asked again): full 1.0 in phase order; the agent loop (Phases 0–1) is the first usable slice; the Wasm target, its host and execution limits (T11) may slip; macOS and Linux required, Windows later; scaled CI volumes with full volumes as a release check; the runtime has zero dependencies and each new compiler crate needs a reason and your approval.

## Q1. Where do the extra translator features belong?

The spec describes explanation levels (§8.2), semantic equivalence checking `rail equiv` (§8.5) and human-readable semantic diffs `rail diff --human` (§8.6), but the roadmap gives them no phase. At handoff you deferred this to now. The agent loop's last step (§9.7) hands a "reviewer packet" in human form to you, which needs the reviewer explanation level and the semantic diff.

A. Split by need: the semantic diff and the `concise` and `reviewer` explanation levels in Phase 1 (they complete the agent loop's review step); `developer` and `audit` levels and `rail equiv` in Phase 5, once the analyses they report on exist
B. All of them in Phase 1
C. All of them in Phase 5
D. All of them may slip (Should, not Must)
X. Other (please specify)

[Answer]: A

## Q2. Are the interactive debugger and the profiler required for 1.0?

The spec's tool list (§9.1) includes `rail debug` (a debugger compatible with the Debug Adapter Protocol, with breakpoints on anchor paths) and `rail prof` (CPU, allocation and reference-counting profiles). The profiler is also how the reference-counting overhead target in §11.2 is measured. No acceptance suite tests the debugger.

A. Profiler is Must (needed for the §11.2 measurement); debugger may slip (Should)
B. Both are Must
C. Both may slip
X. Other (please specify)

[Answer]: A

## Q3. Does the standard library's network module include TLS in 1.0?

The spec puts a TLS 1.3 client and server inside the runtime ("bundled rustls-derived implementation", §5.1). With a zero-dependency runtime, that means building or porting a TLS stack into the runtime, which is large and security-critical work.

A. Yes, TLS 1.3 client and server in `std.net` as the spec says
B. TCP, UDP and DNS in `std.net` for 1.0; TLS may slip (Should) and could later move to a first-party package
C. TLS client only in 1.0; server may slip
X. Other (please specify)

[Answer]: A

## Q4. How should runtime parts the spec calls "derived" from existing libraries be built?

The spec describes the runtime allocator as "mimalloc-derived" and TLS as "rustls-derived". Your practices say the runtime has zero dependencies.

A. Port the needed source into the runtime's own code, keep the original licence notices, and record where each part came from; no crate dependency
B. Write them from scratch, using those libraries only as design references
C. Allow these two as vendored crates in the runtime as a recorded exception
X. Other (please specify)

[Answer]: B

## Q5. Do you approve the large compiler dependencies the spec names?

Your firm rule says each new Rust dependency needs a stated reason and your approval. The spec's design relies on Cranelift (development backend), LLVM (release backend), Wasmtime (sandbox host) and Bitwuzla (a solver for `rail verify --bmc`; it is a C++ library, not a Rust crate). Approving them now lets the requirements state them as constraints; each still gets its own entry when it is added.

A. Approve all four as named in the spec
B. Approve Cranelift and LLVM now; decide Wasmtime and Bitwuzla when their phases start
C. Decide each one when it is added
X. Other (please specify)

[Answer]: A

## Q6. Are the first-party packages part of this work?

The spec keeps the standard library small and says HTTP, compression and databases come as separately versioned first-party packages (`rail/http`, `rail/zstd`, …, §5.1). No acceptance suite needs them.

A. Out of scope for this work; the package system must be able to host them later
B. `rail/http` only, as a real-world test of the package system
C. `rail/http` and `rail/zstd`
X. Other (please specify)

[Answer]: C

## Q7. How much trust infrastructure does the locally runnable registry include?

The spec requires every package to be signed by its publisher and countersigned by a registry transparency log (§5.9 rule 2), and `rail audit` to check an advisory database (§5.9 rule 4). A public registry is out of scope.

A. The local registry includes a working transparency log with inclusion proofs, and `rail audit` reads an advisory database file in a documented format that the registry can serve
B. Publisher signatures only; the transparency log and advisory database are defined interfaces with test implementations
C. Signatures and transparency log fully working; the advisory database may slip
X. Other (please specify)

[Answer]: A

## Q8. If the Wasm sandbox slips, how does agent-written code run?

The spec says agent-generated code runs in the sandbox by default (§4.10), but you allowed the Wasm target and host to slip. Native runs enforce the same capability rules (T12 stays Must) but without hardware isolation.

A. Until the sandbox exists, `rail run` runs natively with capability enforcement, grants nothing unless the run manifest lists it, and prints a clear warning that no sandbox is active
B. Until the sandbox exists, `rail run` refuses code that has not been reviewed and approved (a recorded approval of its `modhash`)
C. The sandbox cannot slip after all; make the Wasm target and host Must
X. Other (please specify)

[Answer]: A

## Q9. Should the work include material that helps models write rAiL?

The spec's risk table (risk 2) suggests "few-shot grammar cards" and constrained-decoding grammars (grammars a model runtime can use to force syntactically valid output) as mitigations for models writing unfamiliar syntax less reliably.

A. Yes, in Phase 1: a grammar card for prompts and a constrained-decoding grammar for the canonical text, both generated from the shared grammar so they cannot drift
B. Grammar card only, in Phase 1
C. Not part of this work
X. Other (please specify)

[Answer]: A

## Q10. Q3 keeps full TLS 1.3 (client and server) and Q4 writes it from scratch. How should that from-scratch TLS be proven safe enough?

A hand-written TLS stack also needs its own cryptography (key exchange, authenticated encryption, certificate checking), which is where subtle, exploitable bugs usually live. The spec's `SEC010` rule and T10 fuzzing cover only part of this.

A. Accept the risk, and require for 1.0: the published test vectors for every cryptographic primitive, interoperability tests against at least two independent TLS implementations, constant-time checks on secret-dependent code, and TLS parsers in the fuzzing set
B. Same as A, and mark TLS as "not for production use" in 1.0 until an outside security review has been done
C. Reconsider: port the cryptography (not the TLS protocol) from an audited library, as a recorded exception to Q4
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Translator features:** the semantic diff (`rail diff --human`) and the `concise` and `reviewer` explanation levels are built in Phase 1 to complete the agent loop's review step; the `developer` and `audit` levels and `rail equiv` are built in Phase 5. [Q1]
- **Debugger and profiler:** `rail prof` is Must, because it measures the reference-counting overhead target; `rail debug` is Should and may slip. [Q2]
- **TLS:** `std.net` includes a TLS 1.3 client and server in 1.0. [Q3]
- **Runtime parts the spec calls "derived":** the allocator and TLS, including its cryptography, are written from scratch, using mimalloc and rustls only as design references. [Q4]
- **TLS verification bar for 1.0:** published test vectors for every cryptographic primitive, interoperability tests against at least two independent TLS implementations, constant-time checks on secret-dependent code, and TLS parsers in the fuzzing set. [Q10]
- **Large compiler dependencies:** Cranelift, LLVM, Wasmtime and Bitwuzla are approved as the spec names them; each still gets its own recorded entry when it is added. [Q5]
- **First-party packages:** `rail/http` and `rail/zstd` are in scope. [Q6]
- **Local registry trust:** a working transparency log with inclusion proofs, and a `rail audit` that reads an advisory database file in a documented format, served by the registry. [Q7]
- **Before the sandbox exists:** `rail run` runs natively with capability enforcement, grants nothing the run manifest does not list, and warns clearly that no sandbox is active. [Q8]
- **Model-writing aids:** Phase 1 includes a grammar card for prompts and a constrained-decoding grammar for canonical text, both generated from the shared grammar. [Q9]

Does this all look correct before I generate the requirements artifact?

Looks correct
Request changes

[Answer]: Looks correct
