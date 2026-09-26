# External Dependency Map — rAiL Toolchain

This maps the things outside the project that a **Bolt** needs. A Bolt is one build pass over one unit of work that ends in something that runs and is tested (see `bolt-plan.md`). The list was confirmed complete by the owner [Q5]. The build is otherwise fully AI-contained: there are no outside teams, hand-offs or approval lead times beyond the owner's own approvals.

| Dependency | Owner | Lead time / availability | Bolts it blocks | If it slips |
|---|---|---|---|---|
| GitHub-hosted macOS and Linux runners | GitHub (project repository) | Available now | Every Bolt (every merge needs CI on both platforms) | Run the same CI on the owner's machines as self-hosted runners; merges wait, rather than skipping a platform (firm rule) |
| Owner's reference machines (macOS AArch64, Linux x86-64) | Owner | Available [brief Q2] | Bolt 8 (first performance numbers), Bolt 10 (measurements), Bolt 15 (full T13), Bolt 17 (sandbox cold start), release checks | Performance results wait; hosted-runner numbers never count as T13 evidence [practices] |
| Model-vendor API access and official tokenizers, at least three model families | Owner (accounts and keys) | Available [brief Q2] | Bolt 10 (measurement tools); final encoding-gate measurements | Measurements wait; keys never enter the repository or saved output |
| Two independent TLS implementations for interoperability tests | Their open-source maintainers; used as test peers only | Available as installable tools; exact choices made in Bolt 12 design | Bolt 12 (crypto-and-net); Bolt 14 uses the result | Bolt 12's done-criteria cannot be met; TLS stays unmerged until they pass |
| Cranelift (pre-approved) | Bytecode Alliance | Available (crate) | Bolt 1, Bolt 7 | Pin a known-good version; each version change is recorded under the dependency rule |
| LLVM (pre-approved) | LLVM project | Available | Bolt 8 | Pin a known-good version; the release backend waits |
| Bitwuzla (pre-approved) | Bitwuzla project | Available (C++ library; a binding needs its own approval) | Bolt 15 (verifier) | The verifier waits; other deep-analysis work proceeds |
| Wasmtime (pre-approved) | Bytecode Alliance | Available (crate) | Bolt 17 (may slip) | Bolt 17 slips, as already allowed |
| Pinned Rust, Go and OCaml toolchains and formatters (benchmark twins) | Their projects | Available | Bolt 3, Bolt 8, Bolt 10, Bolt 15 (T9 differential tests use the Rust twins) | Pin earlier versions; the corpus waits |

## Assumptions & Open Questions

- [assumption] "Available" means the owner already has access. Nothing needs an approval with a lead time.
- Open question: which two TLS implementations to use as interoperability peers. This is decided in Bolt 12's design and needs no new crate if they run as external test programs.
