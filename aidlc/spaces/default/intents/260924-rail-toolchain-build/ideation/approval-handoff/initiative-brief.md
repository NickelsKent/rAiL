# Initiative Brief — rAiL Toolchain

Compiled from the approved Ideation artifacts: `intent-capture/intent-statement.md`, `intent-capture/stakeholder-map.md`, `scope-definition/scope-document.md`, `scope-definition/intent-backlog.md`, plus the answers in `approval-handoff-questions.md` (`[Q<n>]`).

## Intent and Problem

Build rAiL, a programming language designed for AI agents, to the spec at `aidlc/spaces/default/knowledge/documents/rail-language-spec.md`. It addresses four problems: agent code that is ambiguous and hard to review, agent code that is unsafe to run, the untested claim that an agent-first language works better, and the need for a fast compiled target for agents. The trigger is opportunity: agents write more and more code, and no language is designed for them yet. (intent-statement)

## Market Validation

Not part of this plan; market research was not run. The first user is the project owner with their own agents. (intent-statement)

## Feasibility and Risk Highlights

No feasibility assessment was run. The owner has accepted these risks before design starts: [Q1]

| Risk | Accepted | Source |
|------|----------|--------|
| Capacity: full 1.0 is sized in the spec for six to eight engineers over 18–20 months; here one builder with AI agents, no deadline | Yes | [Q1] |
| Late encoding gate: deciding §11.3 after the full toolchain exists may force rework of the text spelling | Yes | [Q1] |
| Performance: reference counting and LLVM work may miss §11.2 targets against Rust, Go and OCaml | Yes | [Q1] |
| Agent reliability: rAiL may not beat Rust's check-pass rate by 10 points | Yes | [Q1] |

Resources: model access and the macOS and Linux reference machines are available, and the cost is acceptable. [Q2]

Mitigation already in the plan: the agent loop (Phases 0–1) is the first usable slice and the fallback if the full 1.0 takes longer than hoped; the Wasm target, its host and the T11 limits may slip. (scope-document)

## Scope Boundary

- **In:** the full 1.0 — all six roadmap phases, with Phase 4 as code only (registry runs locally); the 40-program benchmark corpus in rAiL, Rust, Go and OCaml; model and tokenizer measurements; all acceptance suites automated, scaled in CI with full volumes as a release check. (scope-document)
- **Out:** hosting a public registry; deployment and operations; self-hosting. (scope-document)
- **May slip:** Wasm component target and Wasmtime host; execution limits and audit logging (T11). Capability enforcement (T12) stays Must. (scope-document)
- **Platforms:** macOS and Linux required; Windows later. (scope-document)
- **Translator features** beyond `view` / `absorb` (§8.2, §8.5, §8.6): placement decided in requirements analysis. [Q3]
- **Backlog:** 29 proto-units, IB-01 to IB-29; IB-01 to IB-10 form the first usable slice. (intent-backlog)

## Success Measures

| Measure | Target | Source |
|---------|--------|--------|
| Acceptance suites | All suites for Must items pass on macOS and Linux | scope-document |
| Agent reliability | Check-pass rate at least 10 points higher than Rust's on the same tasks and models | scope-document |
| Performance | §11.2 targets met against Rust, Go and OCaml | scope-document |

## Concept Visuals

Not part of this plan; rough mockups were not run. The value-stream diagram in `scope-document.md` is the only visual. (scope-document)

## Team Plan

One builder — the project owner — working with AI agents, no fixed deadline; the owner decides scope and priority. Future contributors may join later, and the design should make that easy. No progress updates are needed beyond this workflow's approval steps. (intent-statement, stakeholder-map)

## Go / No-Go Recommendation

**Go.** Proceed to requirements and design with the risks above accepted. [Q4]

## Assumptions & Open Questions

- [assumption] Where the translator features (§8.2, §8.5, §8.6) land is open until requirements analysis. [Q3]
