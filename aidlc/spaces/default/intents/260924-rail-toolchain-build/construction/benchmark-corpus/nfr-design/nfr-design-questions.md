# NFR Design — benchmark-corpus (U16) — Questions

The corpus's quality requirements are settled (NFR Requirements). The review of those requirements left two choices that only you can make. The rest of its points become design detail: the per-crate approval of `rayon`'s dependencies at Code Generation, a licence record for the vendored OCaml code, and running the corpus job on every pull request so it can be a required check.

## Q1. How should the I/O programs be confined when they run?

The I/O programs are written by an AI model and run on your Mac and on CI runners. The corpus check gives each program its own run directory, a canary directory and a tree check. That catches writes to the canary and unexpected changes in the run directory, but not reads elsewhere or writes to other places. CI runners are thrown away after each job; your Mac is not. macOS ships a built-in sandbox tool (`sandbox-exec`, deprecated but working). Linux runners can install `bubblewrap`. Neither is installed as a Rust crate.

A. Run every corpus program under an OS sandbox that allows writes only inside its run directory and allows no network: `sandbox-exec` with a fixed profile on macOS, and `bubblewrap` on Linux (installed in CI, and documented for Linux machines). If the sandbox tool is missing, the check refuses to run I/O programs rather than running them unconfined (recommended)
B. No OS sandbox. Record an accepted risk, signed off by you: the programs are reviewed for fairness and approved by you per class, CI runners are disposable, and on your Mac you run the corpus check only after a class is approved
X. Other (please specify)

[Answer]: A

## Q2. What does the 20-minute pull-request budget cover, and what are the hard job limits?

The checker measures its own run, but it cannot enforce a limit if it hangs, and a cold cache (after any toolchain pin or lockfile change) rebuilds all 120 programs.

A. The 20 minutes covers the checker run only, after toolchains are installed and caches restored. The CI job has a hard timeout of 45 minutes per platform on pull requests and 120 minutes nightly, so a hang cannot run on. A cold-cache run that goes over the 20 minutes fails like any other, and the fix is a faster check, never fewer programs (recommended)
B. The 20 minutes covers the whole job, including toolchain install and cache restore
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **Confinement:** every corpus program runs under an OS sandbox that allows writes only inside its run directory and no network: `sandbox-exec` with a fixed profile on macOS, `bubblewrap` on Linux (installed in CI, documented for Linux machines). If the sandbox tool is missing, the check refuses to run the programs rather than running them unconfined (Q1).
- **Budget scope:** the 20 minutes covers the checker run only, after toolchains are installed and caches restored. The CI job has hard timeouts of 45 minutes per platform on pull requests and 120 minutes nightly. A cold-cache run over 20 minutes fails, and the fix is a faster check, never fewer programs (Q2).
- **Design detail from the review (no choice needed):** `rayon`'s resolved dependency crates are listed for your approval at Code Generation before the lockfile is committed; vendored OCaml code gets a recorded licence review; the corpus job runs on every pull request and passes quickly when nothing relevant changed (including `rust-toolchain.toml`), so it can be a required check.

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
