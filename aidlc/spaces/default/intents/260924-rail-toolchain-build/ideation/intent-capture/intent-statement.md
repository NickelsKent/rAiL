# Intent Statement — rAiL Toolchain

## Problem Statement

Build rAiL to the language spec catalogued at `aidlc/spaces/default/knowledge/documents/rail-language-spec.md` (document id `01a0d434-5009-77de-bde3-ca5af460eed1`). [desc]

rAiL is meant to address four problems together: [Q1]

- Code written by AI agents is ambiguous or inconsistent (many ways to write the same thing), which makes it hard to diff, merge and review. [Q1]
- Code written by AI agents is unsafe to run without heavy review; rAiL should provide built-in sandboxing and explicit, limited permissions. [Q1]
- It is not yet shown that a language designed for agents works better than languages designed for humans; this work is also a test of that idea. [Q1]
- Agents need a fast compiled language to target, with performance close to Rust. [Q1]

## Target Customer

The first user is the project owner, working with their own AI coding agents; their needs win when requirements conflict. [Q2]

The owner builds rAiL alone with AI agents for now; contributors may join later, so the design should make it easy for them to do so. [Q12]

## Success Metrics

| Metric | Target | Source |
|--------|--------|--------|
| Delivery scope | The full 1.0 described in the spec: all six phases and all 14 acceptance suites | [Q3] |
| Acceptance suites | The spec's acceptance suites pass in automated tests | [Q4] |
| Agent reliability | Agents write rAiL more reliably than human-oriented languages, measured by the parse-valid and check-pass rates in spec §11.3 | [Q4] |
| Performance | The spec's performance targets in §11.2 are met against Rust, Go and OCaml | [Q4] |

## Initiative Trigger

Opportunity: agents write more and more code, and no language is designed for them yet. [Q5]

## Initial Scope Signal

**Workflow-selected scope:** `rail-toolchain` (workflow-selected). [scope]

**User-confirmed product boundary:**

- The `rail-toolchain` plan is confirmed as the product boundary: agree the goal and scope, design, build and test component by component, and a CI pipeline, with no deployment or operations steps. [Q10]
- The whole 1.0 is delivered in this one workflow, in whatever order the design work suggests. [Q13]
- Phase 4 is built as code only: package manager, signing, and a registry server that can be run locally. Hosting a public registry is out of this work. [Q11]
- Internal inconsistencies in the spec are resolved as decisions during this workflow, and the spec document is updated to match those decisions. [Q9]

**Capacity:** one builder (the owner) working with AI agents, with no fixed deadline. [Q7]

## Assumptions & Open Questions

None.
