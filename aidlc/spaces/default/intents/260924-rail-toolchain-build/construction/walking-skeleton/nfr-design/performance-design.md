# Performance Design — walking-skeleton (U1)

The skeleton has no performance targets (NFR1.1). This design covers only the time limits that stop a stuck process from hanging CI or the protocol server (NFR1.2, NFR1.3), and it makes sure no skeleton timing is ever mistaken for release evidence.

## Sources

- `performance-requirements.md`: NFR1.1–NFR1.3.
- `security-requirements.md`: NFR5.5 (10-second run limit).
- `tech-stack-decisions.md`: CI on GitHub-hosted runners; `cargo-llvm-cov`.

## Time limits

| Limit | Value | Where it is enforced | On expiry |
|---|---|---|---|
| Per test | 60 seconds | `cargo-nextest` profile in `.config/nextest.toml`: `slow-timeout = { period = "30s", terminate-after = 2 }` | The test is killed and reported as failed, with its name |
| Tests that start processes (`rail`, built programs) | 60 seconds | The in-house test helper's process runner, which kills the child and fails the test with the command line | As above, plus the child's captured output |
| End-to-end verification (WF6) | 10 minutes per platform | The CI step's `timeout-minutes: 10` | The job fails, naming the step |
| CI job | 30 minutes (pull request), 60 minutes (nightly, including the 30-minute fuzz run) | The workflow's `timeout-minutes` | The job fails |
| Program run inside `rail` | 10 seconds | The run operation (see `security-design.md`, boundary 4) | `run.failed` ("time limit exceeded") |

**Design decision:** `cargo-nextest` runs the tests in CI and locally. It is a tool installed in CI, not a workspace dependency (like `cargo-llvm-cov`, which works with it). The standard `cargo test` runner has no per-test time limit, and a job-level time limit alone cannot say which test hung. Rejected alternative: a hand-written watchdog thread in every test (more code, and a test stuck in a system call cannot be stopped from inside its own process).

## No performance claims (NFR1.1)

- The skeleton's docs and CI output contain no timing claims.
- CI prints no build-time or start-up measurements, so nothing can be read as evidence for NFR1 or T13. Your answer to the NFR questions (Q5: A) was a hang guard only.

## Deliberately absent

- Caching, incremental checking, parallelism and release builds: they arrive with U4, U5, U6 and U12, where the NFR1 targets apply.
- Profiling hooks: U7 (FR26.3).
