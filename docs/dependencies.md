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
| `libfuzzer-sys` | =0.4.13 | `fuzz/` only | Coverage-guided fuzzing of the in-house JSON parser at the protocol boundary. `fuzz/` is not a workspace member, and CI checks that it never reaches the `rail` dependency tree (NFR7.5). | Approved (NFR requirements Q7) |

The four Cranelift crates share one exact version. They are enough to target
AArch64 macOS and x86-64 Linux: the target is looked up by its fixed triple, so
`cranelift-native` (host CPU detection) is not needed and was not added. Using
Cranelift's baseline features instead of detecting the host CPU also keeps
object files identical across machines of one platform (NFR3.2).

The runtime (`crates/rail-runtime`) has no dependencies. It is a `no_std`
static library that declares the C library's `write` and `exit` itself (no
`libc` crate). The workspace uses no JSON crate (`rail-json` is in-house) and
no test crate (`rail-testkit` is in-house).

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

`Cargo.lock` also lists `serde`, `serde_core` and `stable_deref_trait`. They are
optional dependencies of Cranelift crates, and the lockfile records every
optional dependency. None of them is compiled into `rail` (check with
`cargo tree -p rail --target all -i serde`).

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
| (MIT OR Apache-2.0) AND NCSA | `libfuzzer-sys` (NCSA covers the bundled libFuzzer sources; fuzz crate only) |

The JSONTestSuite files in `tests/data/JSONTestSuite/` are test data under the
MIT licence; see `SOURCE.md` there.

## Tools

These are installed in CI and on developer machines. They are not workspace
dependencies: `cargo-nextest` 0.9.146, `cargo-llvm-cov` 0.9.1, `cargo-deny`
0.20.2, `cargo-fuzz` 0.13.2, `gitleaks` (through `gitleaks/gitleaks-action`),
and the system C compiler driver `cc` for linking.
