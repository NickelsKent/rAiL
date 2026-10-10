<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-26T17:05:00Z — walking-skeleton: every skeleton function is exported; canonical text allows a signature only on exported functions (spec §2.2 'required iff exported') and Q3 requires signatures everywhere, so exporting all functions was the only canonical reading.
- 2026-09-26T17:05:00Z — walking-skeleton: Caps is an opaque, field-less type accepted only as main's parameter; the spec fixes main(caps: Caps) but capabilities arrive with U7.
- 2026-09-26T17:05:00Z — walking-skeleton: the log effect on skel.print_i64 is checked statically only; no log evidence is passed at run time until U6 implements evidence registers (C6).
- 2026-09-29T15:40:00Z — acceptance-harness: treated the walking-skeleton check as suite S0 alongside T1–T14 (Q7); the settings file therefore records 15 suites, not the 14 in spec §12. Volumes that combine a set and a count (T4, T9, T10) are recorded as two entries per level so the ratchet compares each.
- 2026-10-09T06:00:00Z — benchmark-corpus: the 40-program list was put into the consolidated summary so the owner approved it before artifacts were written, rather than only at the Unit checkpoint.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-26T17:05:00Z — walking-skeleton: unsupported constructs are reported under a new temporary rule ID SKL001 rather than an existing spec rule; the spec catalog has no 'not yet supported' rule, and reusing FMT/TY IDs would mislabel toolchain limits as language errors. SKL001 is retired, never reused (BR4.4).
- 2026-09-29T15:40:00Z — acceptance-harness: contract C9 names two volumes (CI and full); the design records three (pull request, nightly, full) per Q3. C9's summary should be updated by its owner (U2) in the code change.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-26T17:05:00Z — walking-skeleton: shadowing and unused named bindings are rejected as SKL001 instead of implementing the real FMT/UNU001 rules now; keeps the skeleton small while never accepting a program it cannot fully check.
- 2026-09-26T17:05:00Z — walking-skeleton: traceability lists all 99 FR IDs with non-skeleton ones marked N/A by owning unit, because the traceability check expects the full requirement set when a unit has no acceptance criteria.
- 2026-09-29T15:40:00Z — acceptance-harness: the firm rule against lowering volumes is enforced mechanically by a ratchet check against main's settings file rather than by review alone; seed and input changes are reported, not refused, because changing them is sometimes legitimate.
- 2026-10-09T06:00:00Z — benchmark-corpus: the corpus check runs as its own CI job rather than inside the acceptance harness; harness suites read corpus/index.toml when their owning units implement them.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-26T17:18:30Z — walking-skeleton: the design check left two non-blocking points for the skeleton checkpoint: when the protocol answers -32602 (bad params) versus rap.unsupported_param is not spelled out in WF5, and NFR6 traces only to BR4.3 although WF6 step 7 also rests on BR5.3/BR5.5/BR6.6.
- 2026-09-26T17:05:00Z — walking-skeleton: fixture file locations, build output directory and the command that starts the protocol server are left to Code Generation; the recorded verification command is chosen at the skeleton checkpoint and must demonstrate WF6.
- 2026-09-29T15:40:00Z — acceptance-harness: no spec/ folder exists in the repository yet, although the team practice says the authoritative spec lives there; the spec updates this unit needs have no home until it does.
- 2026-09-29T15:40:00Z — acceptance-harness: T2 full volume (variants per §2.7 rule) and T10 nightly fuzz duration are left to U3 and U7.
- 2026-09-29T15:46:00Z — acceptance-harness: the second architecture review passed but left four items for code generation: --compare must skip cross-platform suites that are pending or not_run at a level (R-10); the build field is refused by the ratchet yet described as confirmed later by the owning unit (R-11); cross_platform, drivers and uses_seed are not ratcheted (R-12); no settings field holds T10's sanitizer flags (R-13).
- 2026-10-09T06:00:00Z — benchmark-corpus: Go, OCaml, dune and opam are not installed on the owner's Mac; they are needed before code generation.
- 2026-10-09T07:00:00Z — benchmark-corpus: second review passed with open items: OCaml's arm64 backend may fuse a*.b+.c, so OCaml floating-point versions could differ from Rust/Go on Apple silicon and BR2.4 names no OCaml mitigation (R-01); parallel-monte-carlo-pi divides in floating point though flagged integer-only (R-08); BR1.4 field names do not match IndexEntry (R-09).
