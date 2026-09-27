<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-26T17:05:00Z — walking-skeleton: every skeleton function is exported; canonical text allows a signature only on exported functions (spec §2.2 'required iff exported') and Q3 requires signatures everywhere, so exporting all functions was the only canonical reading.
- 2026-09-26T17:05:00Z — walking-skeleton: Caps is an opaque, field-less type accepted only as main's parameter; the spec fixes main(caps: Caps) but capabilities arrive with U7.
- 2026-09-26T17:05:00Z — walking-skeleton: the log effect on skel.print_i64 is checked statically only; no log evidence is passed at run time until U6 implements evidence registers (C6).

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-26T17:05:00Z — walking-skeleton: unsupported constructs are reported under a new temporary rule ID SKL001 rather than an existing spec rule; the spec catalog has no 'not yet supported' rule, and reusing FMT/TY IDs would mislabel toolchain limits as language errors. SKL001 is retired, never reused (BR4.4).

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-26T17:05:00Z — walking-skeleton: shadowing and unused named bindings are rejected as SKL001 instead of implementing the real FMT/UNU001 rules now; keeps the skeleton small while never accepting a program it cannot fully check.
- 2026-09-26T17:05:00Z — walking-skeleton: traceability lists all 99 FR IDs with non-skeleton ones marked N/A by owning unit, because the traceability check expects the full requirement set when a unit has no acceptance criteria.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-26T17:18:30Z — walking-skeleton: the design check left two non-blocking points for the skeleton checkpoint: when the protocol answers -32602 (bad params) versus rap.unsupported_param is not spelled out in WF5, and NFR6 traces only to BR4.3 although WF6 step 7 also rests on BR5.3/BR5.5/BR6.6.
- 2026-09-26T17:05:00Z — walking-skeleton: fixture file locations, build output directory and the command that starts the protocol server are left to Code Generation; the recorded verification command is chosen at the skeleton checkpoint and must demonstrate WF6.
