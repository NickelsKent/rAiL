<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-24T20:42:50Z — Read Q10 B as: Wasm component target, Wasmtime host, and the execution-limit/audit-log sandbox suite (T11) may slip; capability enforcement (T12) stays Must. Confirmed through the consolidated summary rather than a separate question.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-24T20:42:50Z — Added three follow-ups (Q9–Q11): Q4 chose a margin without a number, Q2 let sandboxing slip against the intent's core 'unsafe to run' problem, and Q6 dropped Go/OCaml against the intent's §11.2 success measure. Q11 C supersedes Q6 B.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-24T20:42:50Z — User chose to decide the §11.3 canonical-encoding gate only after the full toolchain exists (Q8 C) rather than at end of Phase 1 as the spec says; accepts rework risk on the text spelling in exchange for measuring on a complete toolchain. Spec relies on the tree/IDs/binary being unaffected by the switch.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
