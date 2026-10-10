# NFR Design — acceptance-harness (U2) — Questions

The harness's quality requirements are settled (NFR Requirements): a 10-minute pull-request budget, 5-hour nightly suite groups, no retries, a scrubbed environment, and results kept 90 days. The review of those requirements left four design choices open. Each changes how the harness behaves, so I'm asking rather than picking.

## Q1. What happens at the moment a run hits its time budget?

The nightly groups have 5-hour budgets inside GitHub's 6-hour job limit. If the harness lets a slow run continue past its budget, GitHub may kill the whole job, and then no results file is written at all.

A. Stop at the budget. The suite that is running is stopped and recorded as `error` ("over time budget"). Suites not yet started are recorded as `not_run` ("budget exhausted"). The results file is still written and the run fails (recommended)
B. Let every suite finish, then fail the run if the total went over budget
X. Other (please specify)

[Answer]: A

## Q2. How are the builds of `rail` treated?

Before any suite runs, the harness builds the debug, release or sanitizer `rail` that the selected suites need. Those `cargo` builds need the normal build environment: `HOME` or `CARGO_HOME` to find the Rust toolchain and crate cache, and on macOS the variables the system linker uses. The scrubbed environment (NFR5.7) is meant for `rail` and the programs it runs.

A. The builds run as ordinary `cargo --locked` builds in the harness's own environment, before the budget clock starts. Only the `rail` processes that suites start get the scrubbed environment, and the budget covers suite time only. CI job limits still cover both (recommended)
B. The builds count against the budget as well
X. Other (please specify)

[Answer]: A

## Q3. Can a suite ever receive a secret from the host?

The scrubbed environment passes no secrets on. Your model measurement tools (U17) need API keys, but they run outside the harness as their own tools; T13 (performance) needs none.

A. Never. The harness passes no host secret to any suite. Tools that need API keys, such as the model measurements, run outside the harness. Changing this later needs a design change you approve (recommended)
B. Allow a suite to name host variables it needs in the settings file, passed on only at the release level on your own machines
X. Other (please specify)

[Answer]: A

## Q4. Which version of the ratchet checks a pull request?

The ratchet stops a pull request from lowering what the suites check. If CI runs the ratchet code from the pull request itself, the same pull request could weaken the ratchet and the settings file together.

A. CI builds `rail-accept` from `main` and runs its ratchet against the pull request's settings file. A change to the ratchet code only takes effect after it is merged, and then checks the next pull request. The pull request that first adds the harness has no base version and passes with a notice (recommended)
B. CI runs the ratchet from the pull request's own code and relies on your review to catch changes to it
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **At the budget:** the harness stops at the budget. The running suite is recorded as `error` ("over time budget"), suites not yet started as `not_run` ("budget exhausted"). The results file is still written and the run fails (Q1).
- **Builds:** the debug, release and sanitizer builds of `rail` run as ordinary `cargo --locked` builds in the harness's own environment, before the budget clock starts. Only the `rail` processes that suites start get the scrubbed environment, and the budget covers suite time only (Q2).
- **Host secrets:** never passed to any suite. Tools that need API keys, such as the model measurements, run outside the harness. Changing this needs a design change you approve (Q3).
- **Ratchet version:** CI builds `rail-accept` from `main` and runs its ratchet against the pull request's settings file. Ratchet changes take effect after merge. The pull request that first adds the harness passes with a notice (Q4).

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
