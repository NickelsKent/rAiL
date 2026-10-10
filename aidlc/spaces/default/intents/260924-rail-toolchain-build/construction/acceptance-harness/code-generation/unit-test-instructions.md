# Unit Test Instructions — acceptance-harness (U2)

## Framework and setup

- Rust's built-in test framework, run with `cargo-nextest` (already used by the workspace).
- Toolchain: the pinned stable in `rust-toolchain.toml`. No nightly is needed for these tests.
- Test files: `crates/rail-testkit/tests/harness_*.rs`. Fixtures: `crates/rail-testkit/tests/data/harness/`. A test-only stand-in for `rail`: `crates/rail-testkit/src/bin/fake-rail.rs`.
- The S0 test (`harness_s0.rs`) builds and runs the real `rail` from the workspace, so it needs the same `cc` the walking skeleton needs.

## Running this unit's tests

The exact unit-scoped command. It runs only the harness test binaries, not the rest of `rail-testkit` or the workspace:

```bash
cargo nextest run --locked -p rail-testkit -E 'binary(/^harness_/)'
```

One file at a time, for example while working on the ratchet:

```bash
cargo nextest run --locked -p rail-testkit --test harness_ratchet
```

This command is runnable before the first Red step: Step 2 of the plan creates the (empty) test files, so it runs zero tests and succeeds.

## Expected coverage

- Test harnesses are excluded from the workspace's 80% line-coverage floor (NFR9), and `rail-testkit` is not in the coverage package list. The harness is still developed tests-first, with every rule in `rules.md` and every requirement in `security-requirements.md` covered by at least one test.
- Standard test strategy: about 5–8 tests per component (settings, ratchet, process runner, scheduler, reporter, comparer, command line, S0). The ratchet and settings components have more, one per rule case.

## Mocking and stubbing

- Suites: tests register fake suites (in test code only) with controllable generators, outcomes, durations and failures. The production registry holds only S0.
- `rail`: tests that exercise the process runner, deadlines, the environment allow-list and group kills use `fake-rail`, never the real compiler.
- `cargo metadata`: the dependency-walk logic is tested on small synthetic JSON documents, plus one test on the real workspace.
- Time budgets: tests pass small budgets (seconds) through a test-only settings file; no test sleeps longer than a few seconds.

## Test data

- Settings files for each validation and ratchet case live in `tests/data/harness/` and are committed. Changing an expected-output file goes through the workspace's explicit bless step (`RAIL_BLESS=1`), reviewed in the pull request.
- Every test creates its own temporary directories with the testkit's `TempDir`; no test writes into the repository.
- Seeds in tests are fixed values, except the one test that checks a fresh seed is printed.
