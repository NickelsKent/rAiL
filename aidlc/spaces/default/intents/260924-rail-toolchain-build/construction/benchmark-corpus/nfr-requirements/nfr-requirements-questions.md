# NFR Requirements — benchmark-corpus (U16) — Questions

Most of the corpus's quality requirements are already settled in Functional Design: byte-identical deterministic output, pinned toolchains, the two approved libraries vendored and checked with `cargo-deny`, I/O confinement, and Rust-reference expected outputs through the bless step. Three things are still open: how long the corpus check may take in CI, how Go and OCaml get installed there, and what to do if OCaml cannot match the other languages' floating-point output.

## Q1. How long may the corpus check take in CI?

The corpus check builds 120 Rust, Go and OCaml programs and runs each on its fixed inputs (twice for data-parallel programs), on macOS and Linux. It runs on pull requests that touch `corpus/` and nightly. The acceptance harness's 10-minute pull-request budget does not cover it, because it is a separate job.

A. At most 20 minutes per platform on pull requests, with build caching keyed on the pinned toolchains and lockfiles. Over budget fails the job, and the fix is to make the check faster, never to drop programs or inputs. Nightly has no separate budget beyond the job limit (recommended)
B. At most 10 minutes per platform, the same as the harness
X. Other (please specify)

[Answer]: A

## Q2. How are Go and OCaml installed in CI?

Your practice pins third-party CI actions to exact commits with read-only permissions. Go has an official setup action. OCaml has a community-maintained setup action, and OCaml packages normally come from opam over the network. The corpus vendors `domainslib`, so builds themselves need no network.

A. Use `actions/setup-go` and `ocaml/setup-ocaml`, each pinned to an exact commit whose SHA is read from GitHub, not from memory. Pin the Go and OCaml versions from `corpus/toolchains.toml`, and install dune at its pinned version (recommended)
B. Download the official Go and OCaml release archives in a script, verify their published checksums, and install them without third-party actions
X. Other (please specify)

[Answer]: A

## Q3. What if OCaml cannot match the other languages' floating-point output?

Three programs compute in floating point (n-body, spectral-norm, parallel-mandelbrot). The design forbids fused multiply-add, and Go has a defined way to prevent it. The OCaml native compiler on Apple silicon may fuse `a *. b +. c` on its own, and it has no switch to turn that off. The spec's T9 compares each rAiL program only with its Rust version, so an OCaml difference does not affect any suite. It affects only the fairness of the comparison.

A. Keep it strict. The OCaml versions are written so each product is bound to its own variable before it is added, and the corpus check must show them byte-identical with Rust on both platforms. If a program still differs, it is a corpus defect: you are told, and the program is reworked before its class is approved (recommended)
B. As A, but if OCaml still differs after one rework, the corpus records that program's OCaml output as its own expected output on that platform and notes the reason in the fairness review. Rust, Go and rAiL stay byte-exact
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- **CI time budget:** the corpus check takes at most 20 minutes per platform on pull requests that touch `corpus/`, with build caching keyed on the pinned toolchains and lockfiles. Going over fails the job, and the fix is a faster check, never fewer programs or inputs. The nightly run is bounded only by the job limit (Q1).
- **Go and OCaml in CI:** installed with `actions/setup-go` and `ocaml/setup-ocaml`, each pinned to an exact commit SHA read from GitHub, using the versions in `corpus/toolchains.toml`, plus dune at its pinned version (Q2).
- **OCaml floating point:** strict. The OCaml float programs bind each product to its own variable before adding, and must be byte-identical with Rust on both platforms. A remaining mismatch is a corpus defect that you are told about, and the program is reworked before its class is approved (Q3).

Does this all look correct before I generate the artifact?

- Looks correct
- Request changes

[Answer]: Looks correct
