# Tech Stack Decisions — benchmark-corpus (U16)

The corpus adds three languages beside Rust, and one small checker. Everything sits under `corpus/`, outside the toolchain's Cargo workspace, and never ships.

## Sources

- `requirements.md`: FR54, NFR3, NFR6, NFR7, NFR10.
- `functional-spec.md`, `rules.md`, `entities.md` (benchmark-corpus Functional Design).
- `contract-summary.md`: C10 corpus layout.
- Functional Design questions [Q4] [Q8] [Q9] and `nfr-requirements-questions.md` [Q1]–[Q3], confirmed.

## Decisions

| Area | Decision | Reason | Alternatives rejected |
|---|---|---|---|
| Rust versions | Pinned stable 1.98.1 (the workspace pin), each program its own Cargo package with a committed lockfile, excluded from the workspace | One pinned stable (team practice); the corpus must not touch the workspace's dependency set | Members of the workspace (would pull corpus crates into `rail`'s supply-chain scope) |
| Go versions | Newest stable Go at Code Generation time, pinned in `corpus/toolchains.toml`; one module per program, standard library only | Spec §11.1 (Go 1.2x); FD Q8 | Unpinned versions |
| OCaml versions | Newest stable OCaml 5.x with dune, pinned in `corpus/toolchains.toml`; one dune project per program | Spec §11.1 (OCaml 5.x); dune is the standard build tool | ocamlfind/Makefiles (more setup, less standard) |
| Parallel libraries | `rayon` (Rust) and `domainslib` (OCaml), data-parallel class only, vendored under `corpus/vendor/` with checksums | Owner approval (FD Q4, Q9) | Third-party libraries in other classes |
| Input generators and checker | Std-only Rust programs in `corpus/<class>/<program>/inputs/gen/` and `corpus/tools/check/`, plus `corpus/tools/helper` for process-pipeline | No new dependency; deterministic on both platforms | Scripts in a separate language (another toolchain to pin) |
| Supply-chain checks | `cargo deny check` over the corpus Rust packages with `corpus/deny.toml`; OCaml vendored sources pinned by checksum | Same hygiene as the workspace (NFR10) | No checks on corpus code |
| CI setup | `actions/setup-go` and `ocaml/setup-ocaml` pinned to exact commit SHAs, read-only permissions | NFR Requirements Q2; team practice on pinned actions | Unpinned actions |

## Dependencies

| Library | Language | Used by | Reason | Approval |
|---|---|---|---|---|
| `rayon` (and its pinned dependencies) | Rust | the ten data-parallel programs | Rust's common data-parallel library, for an idiomatic comparison | Owner, Functional Design Q4 and Q9 |
| `domainslib` (and its pinned dependencies) | OCaml | the ten data-parallel programs | OCaml 5's common task-parallel library | Owner, Functional Design Q4 and Q9 |

Exact versions are taken from crates.io and opam at Code Generation time, not from memory.

## Quality requirements carried by these decisions

- NFR3.8, NFR3.9: byte-identical, deterministic output, including the floating-point rule.
- NFR5.9: I/O confinement in the corpus check.
- NFR6.3: the check runs on macOS and Linux.
- NFR7.8: only the two approved libraries, pinned, vendored and checked.
- NFR9.19, NFR9.20: the 20-minute pull-request budget and a tests-first checker.
- NFR10.5, NFR10.6: pinned setup actions, no secrets, offline builds.

## Assumptions & Open Questions

- [assumption] Go, OCaml, dune and opam will be installed on your Mac before Code Generation; they are not installed now.
