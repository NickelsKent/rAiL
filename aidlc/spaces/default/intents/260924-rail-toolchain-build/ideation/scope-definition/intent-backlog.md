# Intent Backlog — rAiL Toolchain

Proto-units for later design, prioritized with MoSCoW and ordered dependency-first in the spec's phase order. Source tags as in `scope-document.md`: `[intent]`, `[Q<n>]`, `[spec]`.

Prioritization method: MoSCoW for priority (the full 1.0 is in scope, so almost everything is Must [intent]); build order follows dependencies in phase order [Q7]. The first usable slice is IB-01 to IB-10. [Q1]

## Backlog

| ID | Proto-unit | Phase | Priority | Depends on | Acceptance suites | Source |
|----|-----------|-------|----------|------------|-------------------|--------|
| IB-01 | Shared grammar definition that generates the parsers and printers | 0 | Must | — | T1, T4 | [spec] [Q1] |
| IB-02 | Canonical text (`.rlc`) parser and printer, with §2.7 unique-spelling rules and autofix | 0 | Must | IB-01 | T1, T2 | [spec] [Q1] |
| IB-03 | Canonical binary (`.rlb`) codec; text ↔ binary bijection | 0 | Must | IB-01 | T4 | [spec] [Q1] |
| IB-04 | Definition IDs, `defhash` and `modhash` | 0 | Must | IB-02, IB-03 | T6 | [spec] [Q1] |
| IB-05 | Human form (`.rlh`) printer and parser; `view` / `absorb` round trips | 0 | Must | IB-01, IB-04 | T4 | [spec] [Q1] |
| IB-06 | `rail` CLI with `fmt`, `view`, `absorb` | 0 | Must | IB-02, IB-05 | T1, T2, T4 | [spec] [Q1] |
| IB-07 | Query engine: incremental, keyed by `defhash` | 1 | Must | IB-04 | T13 incremental check | [spec] [Q1] |
| IB-08 | Type and effect checker: inference, traits, exhaustiveness | 1 | Must | IB-07 | T9 | [spec] [Q1] |
| IB-09 | Lint framework and diagnostics schema, with `FMT TY FX DEAD UNU` families | 1 | Must | IB-08 | T8 (partial) | [spec] [Q1] |
| IB-10 | Agent protocol server (RAP), structured patches, merge, source maps | 1 | Must | IB-07, IB-09 | T5, T6 | [spec] [Q1] |
| IB-11 | Lowering to Core / Mono / RIR with Perceus reference counting | 2 | Must | IB-08 | T9, T10 | [spec] [intent] |
| IB-12 | Cranelift native backend | 2 | Must | IB-11 | T9 | [spec] [intent] |
| IB-13 | Runtime: allocator, reference counting, system interface | 2 | Must | IB-12 | T10 | [spec] [intent] |
| IB-14 | Standard library `std.core/col/text/fs/json` and test runner | 2 | Must | IB-12, IB-13 | T9, T10 | [spec] [intent] |
| IB-15 | Scheduler, `std.task/par`, deterministic concurrency and replay | 3 | Must | IB-13, IB-14 | T7 | [spec] [intent] |
| IB-16 | Capability-based access for `std.fs/net/proc/env` | 3 | Must | IB-14 | T12 | [Q10] [spec] |
| IB-17 | Execution limits and audit logging for sandboxed runs | 3 | Should (may slip) | IB-15, IB-16 | T11 | [Q10] |
| IB-18 | Wasm component target and Wasmtime host | 3 | Should (may slip) | IB-12, IB-16 | T13 sandbox cold start | [Q2] [Q10] |
| IB-19 | Packages, minimal version selection, lockfile, vendoring, API-diff semver | 4 | Must | IB-06, IB-14 | T14 | [intent] [spec] |
| IB-20 | Package signing and a locally runnable registry server | 4 | Must | IB-19 | T14 | [intent] |
| IB-21 | Reproducible builds and `rail verify` | 4 | Must | IB-12, IB-19 | T3 | [intent] [spec] |
| IB-22 | LLVM release backend and full optimization pipeline | 5 | Must | IB-11 | T13 | [intent] [spec] |
| IB-23 | Security and performance lint families | 5 | Must | IB-09, IB-11 | T8 | [intent] [spec] |
| IB-24 | Bounded model checking verifier for `req` / `ens` | 5 | Must | IB-08 | — | [intent] [spec] |
| IB-25 | Benchmark corpus: 40 programs in rAiL, Rust, Go and OCaml; `rail-perf` harness | 2–5 | Must | IB-14 | T13 | [Q11] [intent] |
| IB-26 | Tokenizer and generation-reliability measurement; §11.3 gate decision after the full toolchain exists | 5 | Must | IB-25, IB-10 | — | [Q8] [Q9] [Q11] |
| IB-27 | Acceptance-suite harness T1–T14: scaled volumes in CI, full volumes as a release check | all | Must | grows with each phase | T1–T14 | [Q5] [intent] |
| IB-28 | Spec document updates for each decision made in this workflow | all | Must | — | — | [intent] |
| IB-29 | Windows support | — | Could (later) | IB-13, IB-15 | T7, T12, T14 on Windows | [Q3] |

## Summary by Priority

| Priority | Items | Source |
|----------|-------|--------|
| Must | IB-01 – IB-16, IB-19 – IB-28 | [intent] [Q10] |
| Should (may slip) | IB-17, IB-18 | [Q2] [Q10] |
| Could (later) | IB-29 | [Q3] |
| Won't (this work) | Public registry hosting; self-hosting | [intent] [spec] |

## Assumptions & Open Questions

- [assumption] Proto-unit boundaries here follow the spec's roadmap and prototype architecture; units generation may merge or split them.
- [assumption] Dependencies between proto-units are read from the spec's prototype architecture diagram and roadmap; domain design should confirm them.
