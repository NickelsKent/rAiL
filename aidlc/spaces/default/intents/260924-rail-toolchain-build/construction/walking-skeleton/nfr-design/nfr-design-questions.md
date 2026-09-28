# NFR Design — walking-skeleton (U1) — Questions

The quality requirements for the skeleton are concrete: limits, time limits, determinism, and failure isolation. Most of the design follows directly from them. Two choices are still yours, because they show up in your repository and your CI.

## Q1. Where should `rail build` put the executables it produces?

Builds write only inside the workspace, and paths in results are workspace-relative (security and reliability requirements). The location should be easy to ignore in version control and should not collide with Cargo's own `target/` directory in this repository.

A. `.rail/build/dev/<module path>`: a hidden `.rail/` directory at the workspace root, which `rail` also uses later for its caches. It is listed in `.gitignore` (recommended)
B. `build/dev/<module path>`: a visible `build/` directory at the workspace root
X. Other (please specify)

[Answer]: A

## Q2. How much fuzzing time should the nightly CI run give the JSON parser?

The fuzz target you approved runs in the nightly CI run. The long fuzz run belongs to the release check.

A. 5 minutes per night, seeded with the JSONTestSuite cases. A crash or hang uploads the failing input as a CI artifact and fails the run, and every finding becomes a committed regression test (recommended)
B. 30 minutes per night, otherwise as A
X. Other (please specify)

[Answer]: B

## Consolidated Summary Confirmation

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
