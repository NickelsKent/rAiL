# Initiative Approval & Handoff — Questions

Inputs: the approved intent statement, stakeholder map, scope document and intent backlog. Market research, feasibility, team formation and rough mockups were not part of this plan, so the questions below cover what the brief needs in their place.

## Q1. The plan has no feasibility step, so no one has checked the scope against capacity. Before design starts, which of these risks do you accept? (select all that apply)

A. Capacity: the full 1.0 is sized in the spec for six to eight compiler engineers over 18–20 months, and here it is one builder with AI agents, no deadline
B. Late encoding gate: deciding the §11.3 canonical-text gate only after the full toolchain exists may force rework of the text spelling (the spec says the tree, IDs and binary are unaffected)
C. Performance: reference-counting overhead and LLVM work may not reach the §11.2 targets against Rust, Go and OCaml
D. Agent reliability: rAiL may not beat Rust's check-pass rate by 10 points
E. None of these; I want them addressed before design starts
X. Other (please specify)

[Answer]: A, B, C, D

## Q2. The measurements need model access (tokenizers and 1,000 generation tasks per form, across at least three model families) and reference machines. What do you have?

A. Model access and the macOS and Linux machines are available; cost is acceptable
B. Model access is available; the Linux machine is not yet (CI or cloud runners will be used)
C. Neither is confirmed yet; confirm before the measurement work starts
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q3. The spec's machine-to-human translator features beyond `view` / `absorb` (explanation levels §8.2, equivalence verification §8.5, translation-difference reports §8.6) have no roadmap phase. Where do they belong?

A. With the human form in the first usable slice (Phases 0–1)
B. After the first usable slice, alongside the checker and agent tooling
C. Leave it to requirements analysis to place them
D. Not yet defined
X. Other (please specify)

[Answer]: C

## Q4. What is your go / no-go decision for moving into requirements and design?

A. Go: proceed to requirements and design with the risks accepted above
B. Go with conditions: proceed, but the conditions I name must be met first (please list them)
C. No-go: stop the workflow here
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- Accepted risks: capacity (full 1.0 with one builder), the late §11.3 encoding gate, the §11.2 performance targets, and the +10-point agent-reliability target (Q1).
- Resources: model access and the macOS and Linux machines are available, and the cost is acceptable (Q2).
- Translator features beyond `view` / `absorb` (§8.2, §8.5, §8.6): requirements analysis decides where they land (Q3).
- Decision: Go — proceed to requirements and design with those risks accepted (Q4).

Does this all look correct before I generate the artifact?

Looks correct
Request changes

[Answer]: Looks correct
