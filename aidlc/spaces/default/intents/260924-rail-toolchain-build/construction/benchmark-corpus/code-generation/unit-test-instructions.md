# Unit Test Instructions — benchmark-corpus (U16)

## Framework and setup

- The corpus checker's own tests: Rust's built-in test framework, std only, in `corpus/tools/check/` (its own package, outside the workspace, with its own lockfile).
- The programs' tests: each program's fixed inputs in `inputs/fixed/` and expected outputs in `expected/`, run by the corpus check.
- Toolchains: Rust 1.98.1 (workspace pin), and Go, OCaml and dune at the versions in `corpus/toolchains.toml`. The sandbox needs `sandbox-exec` (built into macOS) or `bwrap` (Linux).

## Running this unit's tests

The checker's tests (runnable from Step 2 on, before the first Red step):

```bash
cargo test --locked --manifest-path corpus/tools/check/Cargo.toml
```

The corpus check over the programs of one class, or all classes:

```bash
corpus/check.sh --class pure-transform
```

```bash
corpus/check.sh
```

Both commands touch only `corpus/`; neither runs the toolchain workspace's tests.

## Expected coverage

- The corpus and its checker are excluded from the workspace's 80% coverage floor (team practice). The checker is still developed tests-first, with at least one test per rule it enforces (BR1, BR2, BR5, BR6, BR7) and per NFR it carries (NFR3.8, NFR3.9, NFR5.9, NFR9.19).
- Each program has at least one fixed input per listed error case, plus at least one successful input.

## Mocking and stubbing

- The checker's tests use small synthetic corpora with tiny stand-in programs (in Rust, built by the test), including ones that misbehave on purpose: wrong output, thread-dependent output, writing outside the run directory, opening a socket, hanging.
- No test touches the real network or anything outside its temporary directories.

## Test data

- Synthetic corpora live under `corpus/tools/check/tests/data/`.
- Real fixed inputs are committed, at most 64 KiB each and at most eight per program. Large inputs are generated at run time and never committed.
- Expected outputs change only through `corpus/check.sh --bless`, run against the Rust versions, and are reviewed in the pull request.
