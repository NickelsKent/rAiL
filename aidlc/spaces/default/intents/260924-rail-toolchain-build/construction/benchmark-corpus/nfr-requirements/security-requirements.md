# Security Requirements — benchmark-corpus (U16)

The benchmark corpus is project tooling that never ships. Its quality job is to make every comparison trustworthy. Every version of a program must give the same bytes, the build must use only pinned and approved code, and the I/O programs must not touch anything outside their run directory. A unit of kind `spec` produces no separate performance, reliability or observability document, so the corpus check's time budget is recorded here with its NFR ID.

## Sources

- `requirements.md`: FR54 (benchmark corpus), NFR3 (determinism), NFR5 (capability safety), NFR6 (portability), NFR7 (dependencies), NFR9 (code quality, test-first), NFR10 (supply-chain hygiene).
- `functional-spec.md` and `rules.md` (benchmark-corpus Functional Design): workflows WF1–WF4 and rules BR1–BR9, in particular BR2.4 (floating point), BR5 (libraries and toolchains), BR6 (I/O confinement) and BR7 (inputs and outputs).
- `contract-summary.md`: C10 (corpus layout), C9 (suite registration).
- Team practices: Way of Working (CI security checks, pinned actions), Testing Posture, firm rules.
- `nfr-requirements-questions.md` [Q1]–[Q3], confirmed.

## Scope and data classification

| Data | Classification | Where it lives |
|---|---|---|
| Corpus sources, prompts, reviews, fixed inputs, expected outputs | Internal; integrity matters because suites and published comparisons rely on them | `corpus/`, reviewed in pull requests |
| Vendored `rayon` and `domainslib` sources | Third-party, integrity-critical | `corpus/vendor/`, pinned by checksum |
| Generated large inputs | Internal, disposable | Temporary directories at run time, never committed |

No personal data and no secrets. No regulatory framework applies.

## Threat model (STRIDE)

| Threat | Example | Control |
|---|---|---|
| Tampering | A vendored library or a lockfile is changed to pull in different code | Checksums in `corpus/toolchains.toml`; committed lockfiles; `cargo deny check` on the corpus (NFR7.8) |
| Tampering | Expected outputs are edited by hand to make a version pass | Bless-only changes, reviewed in the pull request (BR7.2) |
| Information disclosure / elevation | An I/O program reads or writes outside its run directory on a developer or CI machine | Run directory with current directory, `HOME` and `TMPDIR` inside it, relative paths only, a canary directory and a tree check (NFR5.9) |
| Denial of service | A defective program loops forever and blocks CI | Per-run deadlines in the corpus check; the 20-minute pull-request budget (NFR9.19) |
| Tampering (supply chain) | A third-party CI action is swapped for a malicious version | `actions/setup-go` and `ocaml/setup-ocaml` pinned to exact commit SHAs, with read-only permissions (NFR10.5) |
| Repudiation | A comparison result cannot be traced to the code that produced it | Prompt, model id, date, fairness review and edit log per program (BR3, BR4) |

## Requirements

| ID | Requirement | Verification |
|---|---|---|
| NFR3.8 | Every Rust, Go and OCaml version of a program gives byte-identical stdout, exit code and error line for every fixed input on both platforms, with no tolerance (BR2.2). The three floating-point programs use only correctly rounded binary64 operations in the prompt's order. Go writes products through explicit `float64()` conversions. OCaml binds every product to its own variable before it is added. A remaining OCaml mismatch is a corpus defect, reported to you and reworked before the class is approved. [Q3] | Corpus check on macOS and Linux |
| NFR3.9 | Generators are deterministic for a given seed and size on both platforms, and data-parallel output is the same at 1 thread and at the core count (BR2.3, BR7.1). | Corpus check runs generators twice and data-parallel programs at two thread counts |
| NFR5.9 | Each I/O program runs with its current directory, `HOME` and `TMPDIR` inside a fresh run directory and receives only relative paths. A canary directory beside it must be untouched, and the final tree must match `expected/<input>.tree`. Permission-denied cases are created at run time, and they are reported as skipped, never passed, when running as root (BR6.4, BR6.5). | Corpus check |
| NFR6.3 | The corpus check runs on both macOS and Linux for every pull request that touches `corpus/`, and nightly. | CI configuration review |
| NFR7.8 | Only `rayon` (Rust) and `domainslib` (OCaml) may be used, and only in the data-parallel class. Both are approved by you, pinned to exact versions with their dependencies, vendored under `corpus/vendor/` with checksums in `corpus/toolchains.toml`, and checked by `cargo deny check` (advisories, bans, licences, sources) over the corpus Rust packages using `corpus/deny.toml`. Every corpus Rust package commits its lockfile. Nothing in the corpus is a dependency of the `rail` workspace (BR5.2, BR5.4, BR5.5). | `cargo deny check` in CI; the workspace's own `cargo tree` shows no corpus crate |
| NFR9.19 | On pull requests, the corpus check takes at most 20 minutes per platform, with build caching keyed on the pinned toolchains and lockfiles. Going over fails the job, and the fix is a faster check, never fewer programs or inputs. The nightly run is bounded only by the job limit. [Q1] | The checker measures its run and fails with "over time budget" |
| NFR9.20 | The corpus checker is written tests-first, with its own tests on small synthetic corpora before it runs on the real one. | Its test suite in CI |
| NFR10.5 | CI installs Go and OCaml with `actions/setup-go` and `ocaml/setup-ocaml`, each pinned to an exact commit SHA read from GitHub (not from memory), with read-only default permissions, at the versions in `corpus/toolchains.toml`, plus dune at its pinned version. No secret is needed. [Q2] | CI configuration review; `gitleaks` |
| NFR10.6 | Building and checking the corpus needs no network once toolchains are installed: libraries are vendored, and no corpus program opens a socket (BR6.3). | Corpus check run with Cargo and opam in offline mode |

## Licences

`rayon` and its dependencies, and `domainslib` and its dependencies, must carry licences that fit the project's MIT OR Apache-2.0 choice, checked by `cargo deny` for Rust and recorded in `corpus/toolchains.toml` for OCaml. A licence outside the allowed list is reported to you before anything changes.

## Assumptions & Open Questions

- [assumption] Binding each product to its own variable is enough to stop the OCaml native compiler from fusing a multiply and an add on Apple silicon. If not, the program is reworked (Q3), and you are told.
- [assumption] The Go, OCaml and dune versions, and the setup-action commits, are read from their official sources at Code Generation time.
