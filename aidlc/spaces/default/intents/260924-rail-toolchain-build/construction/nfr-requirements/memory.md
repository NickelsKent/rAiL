<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-26T17:40:00Z — walking-skeleton: derived requirement IDs must inherit an inception NFR ID, so robustness items that come from FR13.1 (server keeps serving, no panics, no partial artifacts) were filed under NFR9 (code quality) rather than inventing a new NFR number.
- 2026-09-29T16:05:00Z — acceptance-harness: NFR sub-IDs continue after the walking skeleton's (NFR3.6, NFR5.7, NFR9.15 …) so IDs stay unique across units.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-26T17:40:00Z — walking-skeleton: the reliability requirements add a ToolError code internal.error and a 10-second run time limit reported as run.failed; neither was in the approved functional design, so NFR Design must carry them into the skeleton's error list.
- 2026-09-29T16:05:00Z — acceptance-harness: a library unit produces no performance/reliability/observability documents, so the harness's time budgets, retries and retention live in security-requirements.md with NFR9/NFR10/NFR3 IDs.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-26T17:40:00Z — walking-skeleton: the runtime is a no_std Rust static library declaring its own write/exit C functions instead of using the libc crate or std, keeping NFR7.1 (C library only) with zero crates at the cost of a little hand-written unsafe.
- 2026-09-29T16:05:00Z — acceptance-harness: the owner chose a 6-hour nightly; to fit GitHub's 6-hour job limit the nightly is split into parallel suite-group jobs with 5-hour budgets rather than shortening the budget.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-26T17:40:00Z — walking-skeleton: Rust is not installed on the owner's Mac; rustup is a prerequisite before Code Generation, and the pinned stable/nightly versions are chosen on that day.
- 2026-09-29T16:12:00Z — acceptance-harness: review passed with open items for NFR Design/code generation: how cargo builds are treated under the scrubbed environment and budgets (R-01); restate the toml/serde isolation check as 'rail-testkit not in rail's dependency closure' since Wasmtime pulls serde (R-02); ratchet trust source (R-03); abort-at-limit vs fail-after and raising the nightly workflow timeout (R-04); a path for suites needing host credentials (R-05).
- 2026-10-09T08:00:00Z — benchmark-corpus: review passed with items for NFR design: define what the 20-minute budget covers and an external job timeout, plus a cold-cache estimate and nightly number (R-01); the STRIDE row overclaims I/O confinement for AI-written programs, needs OS-level confinement or an owner-accepted risk (R-02); rayon's resolved crate list needs owner approval per crate (R-03); OCaml vendored code and toolchain archives lack advisory/licence/checksum checks (R-04); corpus check trigger paths should include rust-toolchain.toml (R-05).
