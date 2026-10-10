# Dependencies

Every third-party crate needs a stated reason and the owner's approval before
it is added (team practice). This file records each approved crate. `deny.toml`
enforces the list: any crate not allowed there fails CI.

## Approved crates

| Crate | Version | Where | Reason | Approval |
|---|---|---|---|---|
| `cranelift-codegen` | =0.136.1 | `rail-codegen` | Dev-mode native code generation for the host (FR23, FR27). Built with `default-features = false` and the `std` and `host-arch` features: no unwinding tables and no timing code. | Pre-approved (requirements Q5) |
| `cranelift-frontend` | =0.136.1 | `rail-codegen` | Builds Cranelift functions from the skeleton's SSA IR. | Pre-approved (requirements Q5) |
| `cranelift-module` | =0.136.1 | `rail-codegen` | Declares and links functions within one module. | Pre-approved (requirements Q5) |
| `cranelift-object` | =0.136.1 | `rail-codegen` | Writes the object file that `cc` links. | Pre-approved (requirements Q5) |
| `toml` | 1.1.7 (features `std`, `serde`, `parse`) | `rail-testkit` only | Read the reviewed suite settings file `tests/acceptance/suites.toml` (acceptance harness, U2). Test-only: `rail-testkit` is never shipped, and the crate never reaches the `rail` binary. | Approved (acceptance-harness Functional Design Q9) |
| `serde` | 1.0.229 (feature `derive`) | `rail-testkit` only | Map the TOML settings onto typed Rust structs with unknown keys rejected (BR1.1). Test-only, as above. | Approved with `toml` (acceptance-harness Functional Design Q9) |
| `libfuzzer-sys` | =0.4.13 | `fuzz/` only | Coverage-guided fuzzing of the in-house JSON parser at the protocol boundary. `fuzz/` is not a workspace member, and CI checks that it never reaches the `rail` dependency tree (NFR7.5). | Approved (NFR requirements Q7) |

The four Cranelift crates share one exact version. They are enough to target
AArch64 macOS and x86-64 Linux: the target is looked up by its fixed triple, so
`cranelift-native` (host CPU detection) is not needed and was not added. Using
Cranelift's baseline features instead of detecting the host CPU also keeps
object files identical across machines of one platform (NFR3.2).

The runtime (`crates/rail-runtime`) has no dependencies. It is a `no_std`
static library that declares the C library's `write` and `exit` itself (no
`libc` crate). The workspace uses no JSON crate (`rail-json` is in-house) and
no test crate (`rail-testkit` is in-house). `toml` and `serde` are dependencies
of `rail-testkit` only; `cargo tree -p rail -e normal --target all -i toml`
prints nothing.

## Transitive crates

The Cranelift family pulls in the crates below, as locked on 2026-09-27. They
are allowed in `deny.toml` as part of the approved family. A version bump that
brings a new transitive crate needs its own entry and approval.

`allocator-api2`, `anyhow`, `arbitrary`, `bumpalo`, `cfg-if`,
`cranelift-assembler-x64`, `cranelift-assembler-x64-meta`, `cranelift-bforest`,
`cranelift-bitset`, `cranelift-codegen-meta`, `cranelift-codegen-shared`,
`cranelift-control`, `cranelift-entity`, `cranelift-isle`, `cranelift-srcgen`,
`crc32fast`, `equivalent`, `fnv`, `foldhash`, `gimli`, `hashbrown` (0.16 and
0.17), `heck`, `indexmap`, `libm`, `log`, `memchr`, `object`, `regalloc2`,
`rustc-hash`, `smallvec`, `target-lexicon`, `wasmtime-internal-core`.

`toml` and `serde` (with `derive`) pull in the crates below in the test-only
`rail-testkit`. Each was approved by the owner on 2026-10-08 with the reason:
test-only, pulled in by the approved `toml`/`serde` (derive) used by
`rail-testkit` to read `tests/acceptance/suites.toml`; never reaches the `rail`
binary.

| Crate | Version | Licence | Pulled in by |
|---|---|---|---|
| `serde_core` | 1.0.229 | MIT OR Apache-2.0 | `serde`, `toml` |
| `serde_derive` | 1.0.229 | MIT OR Apache-2.0 | `serde` (`derive`) |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 | `serde_derive` |
| `quote` | 1.0.47 | MIT OR Apache-2.0 | `serde_derive` |
| `syn` | 3.0.6 | MIT OR Apache-2.0 | `serde_derive` |
| `unicode-ident` | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | `proc-macro2`, `syn` |
| `serde_spanned` | 1.1.2 | MIT OR Apache-2.0 | `toml` |
| `toml_datetime` | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | `toml` |
| `toml_parser` | 1.1.4+spec-1.1.0 | MIT OR Apache-2.0 | `toml` |
| `winnow` | 1.0.4 | MIT | `toml_parser` |

`Cargo.lock` also lists `stable_deref_trait` (an optional dependency of a
Cranelift crate) and `toml_writer` (an optional dependency of `toml`, behind its
`display` feature, which is not enabled). The lockfile records every optional
dependency; neither is compiled. No crate in this section is compiled into
`rail` (check with `cargo tree -p rail -e normal --target all -i serde`).

## Licences

Checked against the published crate metadata on 2026-09-27. All fit rail's
MIT OR Apache-2.0 licence:

| Licence | Crates |
|---|---|
| Apache-2.0 WITH LLVM-exception | the Cranelift crates, `regalloc2`, `target-lexicon`, `wasmtime-internal-core` |
| MIT OR Apache-2.0 (any spelling) | most of the rest |
| MIT | `libm` |
| Unlicense OR MIT | `memchr` (used under MIT) |
| Zlib | `foldhash` |
| (MIT OR Apache-2.0) AND Unicode-3.0 | `unicode-ident` (Unicode-3.0 covers its Unicode data tables; build-time only, through `serde_derive` in `rail-testkit`; approved 2026-10-08) |
| (MIT OR Apache-2.0) AND NCSA | `libfuzzer-sys` (NCSA covers the bundled libFuzzer sources; fuzz crate only) |

The JSONTestSuite files in `tests/data/JSONTestSuite/` are test data under the
MIT licence; see `SOURCE.md` there.

## Tools

These are installed in CI and on developer machines. They are not workspace
dependencies: `cargo-nextest` 0.9.146, `cargo-llvm-cov` 0.9.1, `cargo-deny`
0.20.2, `cargo-fuzz` 0.13.2, `gitleaks` (through `gitleaks/gitleaks-action`),
and the system C compiler driver `cc` for linking.
