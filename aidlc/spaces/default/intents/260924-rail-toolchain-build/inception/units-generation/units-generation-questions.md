# Units Generation — Questions

A unit is a piece of work that goes through Construction on its own: design, then tests and code, then build and test. The 32 building blocks from Domain Design have to be grouped into units. This stage decides which units exist and what each one depends on. The order you build them in is decided in the next planning stage, not here.

Already settled: one `rail` program; the runtime and standard library are linked into compiled rAiL programs; the measurement and acceptance tools are project tooling outside the binary; tests come first; the walking skeleton reaches a native binary on macOS and Linux; Construction runs one unit at a time.

## Q1. How many units should the 32 building blocks become?

Each unit goes through its own design, build and test steps with its own approvals. More units means smaller, easier reviews, but more ceremony for one builder.

A. About 17 units grouped by capability: skeleton, acceptance harness, syntax, checker and lints, agent loop, execution core, system and standard library, concurrency, crypto and TLS, supply chain, first-party packages, release backend and optimizer, deep analysis, debugger, Wasm sandbox, benchmark corpus, measurement tools (recommended)
B. About 30 fine-grained units, roughly one per building block or close pair
C. About 9 coarse units, one per roadmap phase plus the skeleton, the harness and the measurement tooling
X. Other (please specify)

[Answer]: A

## Q2. What should the first unit be, and should the workflow check the skeleton formally?

Your practices say the first thing built is a walking skeleton that reaches a native binary. The workflow has an optional skeleton checkpoint: the first unit is built completely, verified with one command you approve, and signed off by you before any other unit starts. The current workflow settings do not turn that checkpoint on.

A. First unit = the walking skeleton, a thin slice across Syntax, the checker, the service layer, the command line, the protocol server, lowering, the Cranelift backend, the build driver and the runtime core; turn the skeleton checkpoint on (recommended)
B. First unit = the walking skeleton, but keep the checkpoint off and rely on the normal approvals
C. No separate skeleton unit; the first real unit (syntax) comes first
X. Other (please specify)

[Answer]: A

## Q3. Where does the acceptance harness go?

Tests come first, so each phase starts by writing its acceptance suites (T1–T14) so they run and fail.

A. Its own unit right after the skeleton, holding the harness framework, recorded CI volumes and seed policies; each later unit adds its own suites to it (recommended)
B. No separate unit; each unit builds the harness pieces for its own suites
C. One harness unit at the end
X. Other (please specify)

[Answer]: A

## Q4. How strictly should unit dependencies be drawn?

A. Only genuine dependencies, so several build orders stay possible and the planning stage chooses the order (recommended)
B. A single chain in the spec's phase order, so only one order is possible
X. Other (please specify)

[Answer]: A

## Q5. Should the two "may slip" items be their own units?

The debugger and the Wasm sandbox (with execution limits and the audit log) are Should, not Must.

A. Yes: separate units, so either can be dropped or postponed without touching any other unit (recommended)
B. No: fold the debugger into the execution unit and the sandbox into the system unit
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Seventeen units, grouped by capability.** [Q1]
- **The first unit is the walking skeleton.** It is a thin slice across Syntax, the checker, the service layer, the command line, the protocol server, lowering, the Cranelift backend, the build driver and the runtime core. It reaches a native binary on macOS and Linux. The skeleton checkpoint is on: the skeleton is built completely, verified with one command you approve, and signed off before any other unit starts. No settings change is needed for this. The workflow reads the stance from your affirmed practice ("we build a walking skeleton first") when Construction starts. [Q2]
- **The acceptance harness is its own unit, right after the skeleton.** It holds the framework plus each suite's recorded CI volume and seed policy. Every later unit adds its own suites to it first, in line with tests-first. [Q3]
- **Only genuine dependencies are drawn.** Several build orders stay possible, and Delivery Planning chooses among them. [Q4]
- **The debugger and the Wasm sandbox are separate units.** The sandbox unit also holds execution limits and the audit log. Either can slip without touching any other unit. [Q5]

Does this all look correct before I generate the artifact?

Looks correct
Request changes

[Answer]: Looks correct
