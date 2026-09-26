# Risk and Sequencing Rationale — rAiL Toolchain

This document explains why the 17 **Bolts** in `bolt-plan.md` are in their order. A Bolt is one build pass over one unit of work that ends in something that runs and is tested.

## Heuristic used

- **Walking skeleton first** (Cockburn, *Crystal Clear*). A walking skeleton is a thin slice that runs end to end, built first to prove the pieces connect. It is required by the team's practice, and the owner signs it off before any other Bolt starts. [practices]
- **Dependency-first in the spec's phase order** for everything after the skeleton [scope Q7] [Q1]. No formal scoring model is used. A Weighted Shortest Job First model (which ranks work by value, urgency and risk reduction divided by size) was offered and declined [Q1]. The reason: nearly every unit is a Must, value only arrives once the dependencies below it exist, and phase order already encodes those dependencies.
- **Targeted risk-first moves** (Boehm, spiral model). Three risks are pulled as early as the dependencies allow [Q4].
- **"May slip" units last** [Q3]. The debugger and the Wasm sandbox come after every Must unit, so dropping or postponing them delays nothing.

## Where the order departs from strict phase order

| Move | Phase-order position | Chosen position | Why | Allowed by the unit dependencies? |
|---|---|---|---|---|
| benchmark-corpus before syntax | Measurement work, late | Bolt 3 | Required, not chosen: the T4 round trip in syntax, and every later corpus-based suite, need the corpus to exist (Units Generation revision) | Yes. Syntax depends on it |
| release-backend right after execution-core | Phase 5, near the end | Bolt 8 | Performance risk: the §11.2 targets (≤1.3× Rust, reference-counting overhead ≤8%) are the costliest to miss late. If the design can't reach them, the evidence arrives while the standard library and later units can still adapt [Q4-A] | Yes. It depends only on execution-core |
| measurement-tools right after system-and-stdlib | Phase 5 / after full toolchain | Bolt 10 | Model-reliability risk: the core claim (rAiL at least 10 points above Rust's check-pass rate) is the project's reason to exist. Measuring early shows whether the design is on track. The encoding gate still waits for the full toolchain [Q4-B] [scope Q8] | Yes. It depends on system-and-stdlib and benchmark-corpus |
| crypto-and-net immediately after concurrency | Phase 3 | Bolt 12 | TLS from scratch is the highest-risk code in the project. This confirms it is not pushed later [Q4-C] | Yes. It depends on concurrency |
| debugger and wasm-sandbox last | Phase 2 and Phase 3 | Bolts 16–17 | Should items; placed so slipping them blocks nothing [Q3] | Yes. Nothing depends on them |

Every Bolt comes after all the units it depends on (checked against the edge block in `unit-of-work-dependency.md`). No move required a new dependency.

## Risk register

| # | Risk | Likelihood | Impact | Mitigation in this plan |
|---|---|---|---|---|
| R1 | Capacity: full 1.0 was sized for six to eight engineers over 18–20 months; here there is one builder | High | High | Agent loop (Bolt 6) is a usable fallback product; slip units last; one-unit Bolts keep each review small [brief] |
| R2 | Performance targets missed because of reference counting or monomorphization | Medium | High | Release backend pulled to Bolt 8 for early numbers; profiler in Bolt 9 attributes RC cost; full T13 in Bolt 15 [Q4-A] |
| R3 | Agents write rAiL less reliably than Rust | Medium | Critical | Early measurements in Bolt 10; grammar card and constrained-decoding grammar in Bolt 4; the encoding gate remains available [Q4-B] |
| R4 | From-scratch TLS or crypto has exploitable defects | Medium | High | Its own Bolt (12) with test vectors, two-way interoperability, constant-time checks and fuzzing |
| R5 | Late encoding decision forces rework of the text spelling | Medium | Medium | Everything operates on the tree, not the spelling; the grammar card and decoding grammar regenerate from the shared grammar [scope Q8] |
| R6 | Corpus rAiL programs written before the language exists turn out invalid | Medium | Low | Each later unit's suites validate them; an invalid program is a defect fixed with a reproducing test |
| R7 | macOS CI runners are slow or costly | Medium | Low | Three CI levels; heavy work on the owner's machines [practices] |
| R8 | Wasm sandbox slips, so agent code runs without hardware isolation | Medium | Medium | Native runs enforce capabilities and warn clearly (FR35); T12 stays Must |

## Assumptions & Open Questions

- [assumption] The early numbers from Bolts 8 and 10 are directional only. The official results come from the full toolchain.
- Open question: if Bolt 8 or Bolt 10 shows a large gap to target, whether to pause and revisit design before Bolt 11. That decision is made at those Bolts' checkpoints.
