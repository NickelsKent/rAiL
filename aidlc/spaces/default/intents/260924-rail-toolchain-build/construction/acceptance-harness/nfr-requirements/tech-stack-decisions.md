# Tech Stack Decisions — acceptance-harness (U2)

The harness extends the workspace the walking skeleton created. It adds no new language, tool or runtime. It adds one approved pair of test-only crates and one program. Everything else follows the skeleton's decisions and the affirmed practices.

## Sources

- `requirements.md`: FR57 (acceptance harness), NFR7 (dependencies), NFR9 (code quality), NFR10 (supply-chain hygiene), C1 (Rust, one Cargo workspace, pinned toolchains).
- `functional-spec.md` and `rules.md` (acceptance-harness Functional Design): the `rail-accept` program, the settings file, builds per suite (BR3.6), the registry (BR5.1), reporting (BR7).
- `contract-summary.md`: C9 (suite registration; amended in the harness pull request per BR9.2), C10 (corpus layout).
- Functional Design questions [Q1] [Q2] [Q9] and `nfr-requirements-questions.md` [Q1]–[Q7], confirmed.

## Decisions

| Area | Decision | Reason | Alternatives rejected |
|---|---|---|---|
| Where the harness lives | A `harness` module and a `suites` module in `crates/rail-testkit`, plus a `rail-accept` binary (`crates/rail-testkit/src/bin/rail-accept.rs`). Never shipped. (FD Q2) | The testkit already has the process runner, temporary directories and the protocol client, and was marked for extension by U2 | A separate crate (more wiring for no gain); plain `cargo test` suites (no level selection, no results file) |
| Settings format | TOML at `tests/acceptance/suites.toml`, read with `toml` + `serde` (FD Q1, Q9) | Owner's choice; easy to review by hand in pull requests | JSON with `rail-json` (no new crate, but harder to review); canonical rAiL text (does not exist yet) |
| Results format | JSON written with the in-house `rail-json` writer, which keeps field order and emits no whitespace, so output is byte-deterministic (BR7.2) | Already in the workspace; deterministic by construction | `serde_json` (another crate, and its map ordering would need care) |
| Toolchains | Pinned stable (`rust-toolchain.toml`, currently 1.98.1) builds and runs the harness and the debug and release builds of `rail`. The pinned nightly builds only the sanitizer build for T10 and the fuzz targets (BR3.6) | Team practice: one pinned stable; nightly only for fuzzing, sanitizers and `miri` | Building the harness with nightly (nothing the harness itself needs) |
| Randomness for fresh seeds | Read 8 bytes from the operating system's random source (`/dev/urandom` on macOS and Linux) through the standard library's file API | No crate needed; both platforms provide it | The `rand` or `getrandom` crates (not needed for one seed per run) |
| Input generation | Each suite's generator uses a small in-house deterministic generator seeded by the run seed (a counter-based mixer, such as SplitMix64), written in `rail-testkit` | Inputs must depend only on seed and volume, identically on both platforms (BR4.3); no crate needed | `rand` with `StdRng` (its algorithm may change between versions, which would change inputs for the same seed) |
| Process control | `std::process::Command` with an explicit argument list and a cleared environment plus the allow-list (NFR5.7); the testkit's existing deadline handling, made configurable per suite (BR5.2) | Never through a shell; no secrets passed on | Shell scripts per suite |

## Dependencies

| Crate | Kind | Used by | Reason | Approval |
|---|---|---|---|---|
| `toml` | test-only (normal dependency of `rail-testkit`, which is never shipped) | the settings reader | Read `tests/acceptance/suites.toml` | Approved by the owner, Functional Design Q9 |
| `serde` (with its derive feature) | test-only, as above | typed settings structs | Map the TOML settings onto typed Rust structs with unknown keys rejected (BR1.1) | Approved with `toml`, Functional Design Q9 |
| `rail-json` | workspace crate | results writer, RAP message reader | Deterministic JSON; allowed by BR5.1 | Existing |

Versions: the newest release of each crate at the time of Code Generation, confirmed on crates.io, not from memory, and pinned by `Cargo.lock`. Their transitive crates (for example the proc-macro crates behind `serde`'s derive support) must pass `cargo-deny`'s licence, advisory, ban and source checks. A transitive crate that does not pass is reported to you before anything changes in `deny.toml`.

`rail-testkit` must not depend on any compiler crate (BR5.1). A test checks this.

## Quality requirements carried by these decisions

- NFR7.7: `toml` and `serde` never reach the `rail` binary (see `security-requirements.md`).
- NFR3.6, NFR3.7: no retries, deterministic results.
- NFR5.7, NFR5.8: scrubbed environment and a fresh per-run directory.
- NFR9.15–NFR9.18: the ratchet on every pull request, and the time budgets for pull-request and nightly runs.
- The harness follows the workspace rules: Rust 2024 edition, `rustfmt` defaults, `clippy` with warnings as errors, `#![forbid(unsafe_code)]`, and no printing from library code (only `rail-accept` prints).
- Coverage: test harnesses are excluded from the 80% floor (NFR9), but the harness is still written tests-first and its own tests run in CI.

## Assumptions & Open Questions

- [assumption] SplitMix64 (or an equivalent fixed, documented mixer) is good enough to spread generated inputs. It does not need to be cryptographically strong, only fixed across versions and platforms.
- [assumption] `/dev/urandom` exists on every macOS and Linux machine the harness runs on.
