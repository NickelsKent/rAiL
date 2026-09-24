# Intent Capture & Framing — Questions

## Sources

- [desc] Initial description: "Build rAiL to the language spec catalogued at aidlc/spaces/default/knowledge/documents/rail-language-spec.md (document id 01a0d434-5009-77de-bde3-ca5af460eed1)"
- [scope] Workflow-selected scope: `rail-toolchain`.

## Q1. What problem should rAiL solve for you? (select all that apply)

The spec argues that languages designed for humans make AI agents' code hard to generate reliably, review and run safely. Which of these problems are your reason for building it?

A. Agents produce code that is ambiguous or inconsistent (many ways to write the same thing), which makes their output hard to diff, merge and review
B. Agent-written code is unsafe to run without heavy review; I want built-in sandboxing and explicit, limited permissions
C. I want to test whether an agent-first language actually works better (research / proof of the idea)
D. I want a fast compiled language that agents can target, with performance close to Rust
E. Not yet defined
X. Other (please specify)

[Answer]: A, B, C, D

## Q2. Who is rAiL for first?

This decides whose needs win when requirements conflict.

A. Me — I'll use it with my own AI coding agents
B. My team or organisation's AI agents and developers (internal use)
C. External developers and the open-source community
D. Builders of AI agent platforms who want a safe language for agents to target
E. Not yet defined
X. Other (please specify)

[Answer]: A

## Q3. What does a successful outcome of this piece of work look like?

The spec's roadmap has six phases over roughly 18–20 months for a team of six to eight compiler engineers. This workflow can aim at a smaller first milestone.

A. Phase 0, Foundations: the three file forms (canonical text, binary, human view), lossless conversion between them, definition IDs and hashes, and the `fmt` / `view` / `absorb` commands (spec acceptance suites T1, T2 and T4 pass)
B. Phases 0–1: the above plus type and effect checking, the lint framework, the agent protocol server, structured patches and merges (adds suites T5 and T6, and the token-cost decision in spec §11.3)
C. Phases 0–2: the above plus compiling and running real programs natively with the core standard library (adds suites T9 and T10)
D. The full 1.0 described in the spec (all six phases, all 14 acceptance suites)
E. Not yet defined
X. Other (please specify)

[Answer]: D

## Q4. Which measures will tell you it worked? (select all that apply)

A. The spec's acceptance suites for the chosen phases pass in automated tests
B. Agents write rAiL more reliably than human-oriented languages (the parse-valid and check-pass rates in spec §11.3)
C. The spec's performance targets (§11.2) are met against Rust, Go and OCaml
D. Usage: a set number of real programs or people using it (please give the number)
E. Not yet defined
X. Other (please specify)

[Answer]: A, B, C

## Q5. Why start this now?

A. Opportunity: agents write more and more code, and no language is designed for them yet
B. Pain: agent-generated code in my current work is hard to review, merge or run safely
C. Personal or research interest in language design for agents
D. A strategic bet for a product or company
E. Not identified
X. Other (please specify)

[Answer]: A

## Q6. Who decides scope and priorities, and who else has a say?

A. Only me: I decide and I build it (with AI agents)
B. I decide; other people build or contribute (you can name them later)
C. Several decision-makers in a team or organisation (please name their roles)
D. I decide, but an open-source community's input shapes priorities
E. Not yet defined
X. Other (please specify)

[Answer]: B

## Q7. Who will build it, and on what timeline?

The spec sizes the full 1.0 for six to eight compiler engineers over 18–20 months, so capacity decides how much this work can realistically take on.

A. Me alone, working with AI agents, with no fixed deadline
B. Me alone, working with AI agents, with a target date (please give it)
C. A small team (two to five people) working with AI agents
D. A team of six to eight, as the spec assumes
E. Not yet defined
X. Other (please specify)

[Answer]: A

## Q8. Does anyone need progress updates, and how often?

A. No: the approval steps in this workflow are enough
B. A written summary at the end of each major phase
C. Regular updates (for example weekly) to specific people
D. Public progress (for example a changelog or README in the repository)
E. Not applicable
X. Other (please specify)

[Answer]: A

## Q9. The spec has some internal inconsistencies. How should they be handled?

About nine items. For example: the spec plans to produce WebAssembly with Cranelift, which Cranelift cannot do; `>>` means both function composition and bit-shift in the human form; and converting edited human text back to canonical form is not deterministic without the previous version of the module.

A. Resolve them as decisions during requirements and design; the spec document stays as it is
B. I will revise the spec document first, then this work continues from the new version
C. Resolve them in this workflow, and also update the spec document to match the decisions
D. Keep the spec as-is and only raise an issue when it blocks work
E. Not yet defined
X. Other (please specify)

[Answer]: C

## Q10. Does the workflow's plan match what you want built?

This workflow was set up with the `rail-toolchain` plan: agree the goal and scope, design, build and test component by component, and a CI pipeline, with no deployment or operations steps.

A. Yes: confirm the `rail-toolchain` plan as the product boundary
B. Yes for this workflow, but the product boundary is narrower: only the phases chosen in Q3
C. No: the product boundary is wider and also needs packaging, signing and a package registry (spec Phase 4), which would bring back release steps
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q11. Full 1.0 includes Phase 4 (packaging, signing and a package registry), but the confirmed plan has no release or deployment steps. How should Phase 4 be handled?

Q3 chose the full 1.0; Q10 confirmed a plan with no deployment or operations steps.

A. Build Phase 4 as code only (package manager, signing, a registry server that can be run locally); hosting a public registry is out of this work
B. Leave Phase 4 out of this work; the product boundary is Phases 0–3 and 5
C. Include hosting a public registry, and add the release and deployment steps back to the plan
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q12. Q6 says other people will build or contribute, but Q7 says you will build it alone. Which is true for this work?

A. I build it alone with AI agents now; contributors may join later, and the design should make that easy
B. I build it alone with AI agents; no other contributors are planned
C. Others will contribute from the start (you can name them or their roles)
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q13. The full 1.0 is sized at 18–20 months for a team of six to eight. With one builder and no deadline, how should this work be sequenced?

A. One workflow for the whole 1.0, delivered and checked in the spec's phase order (each phase usable before the next starts)
B. This workflow plans the whole 1.0 but builds only Phase 0 first; later phases each get their own follow-on piece of work
C. One workflow for the whole 1.0, in whatever order design work suggests
D. Not yet defined
X. Other (please specify)

[Answer]: C

## Consolidated Summary Confirmation

- Problem: agent code that is ambiguous and hard to review, agent code that is unsafe to run, testing whether an agent-first language works better, and a fast compiled target for agents (Q1).
- First audience: you, with your own AI coding agents (Q2).
- Success for this work: the full 1.0 in the spec (Q3), measured by the acceptance suites passing, agents writing rAiL more reliably (spec §11.3), and the §11.2 performance targets (Q4).
- Trigger: opportunity, since agents write more and more code and no language is designed for them (Q5).
- Decisions: you decide scope and priority (Q6); you build alone with AI agents now, and the design should make it easy for contributors to join later (Q12).
- Capacity: you alone with AI agents, no fixed deadline (Q7); one workflow for the whole 1.0, in whatever order the design suggests (Q13).
- Updates: none beyond this workflow's approval steps (Q8).
- Spec inconsistencies: resolved as decisions in this workflow, and the spec document is updated to match (Q9).
- Product boundary: the `rail-toolchain` plan is confirmed (Q10); Phase 4 is built as code only, including a registry server you can run locally, and hosting a public registry is out of this work (Q11).

Does this all look correct before I generate the artifact?

Looks correct
Request changes

[Answer]: Looks correct
