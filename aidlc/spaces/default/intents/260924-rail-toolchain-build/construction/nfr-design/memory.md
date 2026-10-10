<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
- 2026-09-26T22:45:00Z — walking-skeleton: settled the functional design's open point on -32602 versus rap.unsupported_param in the security design: -32602 when params are missing or mistyped, rap.unsupported_param when well-formed but asking for something the skeleton does not do.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
- 2026-09-29T21:10:00Z — acceptance-harness: NFR Design added two settings fields not in the Functional Design entities (a [budgets] table and a per-suite nightly_group) so budgets and nightly groups live in the ratcheted settings file rather than in workflow YAML; code generation must add them to entities and the C9 amendment.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
- 2026-09-26T22:45:00Z — walking-skeleton: chose cargo-nextest (a CI tool, not a crate) to enforce the 60-second per-test limit, because the standard test runner has no per-test timeout and a job-level limit cannot name the hung test.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
- 2026-09-26T22:45:00Z — walking-skeleton: build.no_entry is mapped to command-line exit 1 (a blocking problem in the user's module) rather than 3; worth confirming at the skeleton checkpoint.
- 2026-09-29T21:10:00Z — acceptance-harness: second review suggested (non-blocking) a grandchild-kill test with a rail stand-in and an error for a missing results file; carry into the code-generation plan.
- 2026-10-09T09:00:00Z — benchmark-corpus: review passed with items for code generation: Linux bwrap binds root read-only so reads are not confined (unlike macOS), and GitHub ubuntu runners may block the user namespaces bwrap needs, which would fail the required check closed (R-01); sandbox wrapper must keep the HOME/TMPDIR/relative-path rules and root-skip logic (R-02); builds run unsandboxed (R-03); no per-program deadline value and no die-with-parent on macOS (R-04); bubblewrap package unpinned (R-05).
