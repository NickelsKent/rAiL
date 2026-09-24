# Decision Log — Ideation

All decisions made by the project owner during Ideation (2026-09-24), with the question that recorded each one.

| # | Decision | Stage | Source |
|---|----------|-------|--------|
| D-01 | rAiL addresses four problems: ambiguous agent code, unsafe-to-run agent code, testing the agent-first idea, and a fast compiled target | Intent capture | intent-capture Q1 |
| D-02 | The first user is the project owner with their own AI agents | Intent capture | intent-capture Q2 |
| D-03 | This work delivers the full 1.0 (all six phases, all acceptance suites) | Intent capture | intent-capture Q3 |
| D-04 | Success is measured by acceptance suites, agent reliability (§11.3) and performance (§11.2) | Intent capture | intent-capture Q4 |
| D-05 | Trigger: opportunity — no language is designed for agents yet | Intent capture | intent-capture Q5 |
| D-06 | The owner decides scope and priorities; builds alone with AI agents now; design should make it easy for contributors to join later | Intent capture | intent-capture Q6, Q12 |
| D-07 | One builder, no fixed deadline | Intent capture | intent-capture Q7 |
| D-08 | No progress updates beyond this workflow's approval steps | Intent capture | intent-capture Q8 |
| D-09 | Spec inconsistencies are resolved as decisions in this workflow and written back into the spec | Intent capture | intent-capture Q9 |
| D-10 | The `rail-toolchain` plan (no deployment or operations steps) is the product boundary | Intent capture | intent-capture Q10 |
| D-11 | Phase 4 is code only; the registry runs locally; hosting a public registry is out | Intent capture | intent-capture Q11 |
| D-12 | One workflow for all of 1.0 | Intent capture | intent-capture Q13 |
| D-13 | First usable slice: the agent loop (Phases 0–1) | Scope definition | scope-definition Q1 |
| D-14 | May slip: Wasm component target, Wasmtime host, execution limits and audit logging (T11); capabilities (T12) stay Must | Scope definition | scope-definition Q2, Q10 |
| D-15 | macOS and Linux are must-have; Windows later | Scope definition | scope-definition Q3 |
| D-16 | Agent-reliability target: check-pass rate at least 10 points above Rust's | Scope definition | scope-definition Q4, Q9 |
| D-17 | Acceptance volumes scaled in CI; full volumes as a release check | Scope definition | scope-definition Q5 |
| D-18 | Benchmark corpus in rAiL, Rust, Go and OCaml, plus model and tokenizer measurements, are in this work | Scope definition | scope-definition Q11 (supersedes Q6) |
| D-19 | Build order follows dependencies in phase order where the design allows a choice | Scope definition | scope-definition Q7 |
| D-20 | The §11.3 canonical-encoding gate is decided after the full toolchain exists | Scope definition | scope-definition Q8 |
| D-21 | Capacity, late encoding gate, performance and agent-reliability risks are accepted | Approval & handoff | approval-handoff Q1 |
| D-22 | Model access and macOS / Linux machines are available; cost is acceptable | Approval & handoff | approval-handoff Q2 |
| D-23 | Placement of translator features §8.2, §8.5, §8.6 is left to requirements analysis | Approval & handoff | approval-handoff Q3 |
| D-24 | Go: proceed to requirements and design | Approval & handoff | approval-handoff Q4 |

## Spec Changes Pending

Per D-09, these decisions are to be written into the spec document: D-15 (platforms), D-17 (acceptance volumes), D-20 (gate timing), D-16 (cross-language reliability target), D-14 (Phase 3 exit criterion). (scope-document)

## Assumptions & Open Questions

- [assumption] D-23 remains open until requirements analysis places the translator features.
